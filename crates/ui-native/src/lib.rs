//! The native settings panel. A front-end like the Svelte one: it renders
//! [`Event`]s and answers with [`Command`]s, and knows nothing else of the
//! world. All of egui lives in this crate; a shell hands it window events, a
//! place to paint, and takes the commands back.

mod paint;

use client::{Command, Effects, Event, ToneMap, Tool};
use winit::window::Window;

pub struct Panel {
    context: egui::Context,
    /// Absent without a window: a headless picture of the panel.
    input: Option<egui_winit::State>,
    painter: paint::Painter,
    open: bool,
    /// What the client last said is in force: the panel edits a copy of it.
    effects: Effects,
    fps: f32,
    building: Building,
}

/// The tool in hand and the paints on offer, as the client last said.
#[derive(Clone, Debug)]
struct Building {
    tool: Option<Tool>,
    /// The tool put down last, which building again takes.
    last: Tool,
    paint: u8,
    palette: Vec<egui::Color32>,
    /// Whether there is a stroke to take back, and one to put back.
    history: (bool, bool),
}

impl Default for Building {
    fn default() -> Self {
        Building {
            tool: None,
            last: Tool::Create,
            paint: 0,
            palette: Vec::new(),
            history: (false, false),
        }
    }
}

impl Panel {
    /// `format` is the format of the targets the panel is painted over.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, window: &Window) -> Panel {
        let context = egui::Context::default();
        let input = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        Panel {
            input: Some(input),
            ..Panel::headless(device, format)
        }
        .sharing(context)
    }

    /// A panel with no window: nothing to click, but [`Panel::picture`] shows
    /// what it looks like.
    pub fn headless(device: &wgpu::Device, format: wgpu::TextureFormat) -> Panel {
        Panel {
            context: egui::Context::default(),
            input: None,
            painter: paint::Painter::new(device, format),
            open: false,
            effects: Effects::default(),
            fps: 0.0,
            building: Building::default(),
        }
    }

    fn sharing(mut self, context: egui::Context) -> Panel {
        self.context = context;
        self
    }

    /// A window event, before the shell acts on it. True when the panel took
    /// it: a click on a slider is not a click on the world.
    pub fn window_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        self.input
            .as_mut()
            .is_some_and(|input| input.on_window_event(window, event).consumed)
    }

    /// The pointer now steers the camera: nothing in the panel is hovered.
    pub fn pointer_gone(&mut self) {
        if let Some(input) = &mut self.input {
            input.egui_input_mut().events.push(egui::Event::PointerGone);
        }
    }

    /// What the client said this frame.
    pub fn event(&mut self, event: &Event) {
        match event {
            Event::EffectsChanged { effects } => self.effects = *effects,
            Event::Stats { fps, .. } => self.fps = *fps,
            Event::ToolChanged { tool, paint } => {
                self.building.tool = *tool;
                self.building.last = tool.unwrap_or(self.building.last);
                self.building.paint = *paint;
            }
            Event::History { undo, redo } => self.building.history = (*undo, *redo),
            Event::Palette { colors } => {
                self.building.palette = colors.iter().filter_map(|hex| color(hex)).collect();
            }
            _ => {}
        }
    }

    /// Lays the panel out, paints it over `target` and returns what the person
    /// asked for.
    pub fn frame(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
    ) -> Vec<Command> {
        let raw = match &mut self.input {
            Some(input) => input.take_egui_input(window),
            None => egui::RawInput::default(),
        };
        let mut effects = self.effects;
        let (mut open, fps) = (self.open, self.fps);
        let mut asked = None;
        let output = self.context.run_ui(raw, |root| {
            layout(root, &mut open, &mut effects, fps);
            asked = build_layout(root, &self.building);
        });
        self.open = open;
        if let Some(input) = &mut self.input {
            input.handle_platform_output(window, output.platform_output);
        }
        self.paint(
            device,
            queue,
            encoder,
            target,
            size,
            output.shapes,
            &output.textures_delta,
        );

        let mut commands: Vec<Command> = asked.into_iter().collect();
        if effects != self.effects {
            // Shown at once; the client's answer, clamped, replaces it next frame.
            self.effects = effects;
            commands.push(Command::SetEffects { effects });
        }
        commands
    }

    /// The panel, open, over a picture of `size`: how a headless shot looks at
    /// it. Laid out over a few seconds of egui's own clock: it sizes a window
    /// on the pass after it first sees it, and fades it in.
    pub fn picture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
    ) {
        let (mut open, mut effects, fps) = (true, self.effects, 60.0);
        let mut textures = egui::TexturesDelta::default();
        let mut shapes = Vec::new();
        for second in 0..3 {
            let raw = egui::RawInput {
                time: Some(f64::from(second)),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(size[0] as f32, size[1] as f32),
                )),
                ..Default::default()
            };
            let output = self
                .context
                .run_ui(raw, |root| layout(root, &mut open, &mut effects, fps));
            textures.append(output.textures_delta);
            shapes = output.shapes;
        }
        self.paint(device, queue, encoder, target, size, shapes, &textures);
    }

    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
        shapes: Vec<egui::epaint::ClippedShape>,
        textures: &egui::TexturesDelta,
    ) {
        let pixels_per_point = self.context.pixels_per_point();
        let primitives = self.context.tessellate(shapes, pixels_per_point);
        self.painter.paint(
            device,
            queue,
            encoder,
            target,
            size,
            pixels_per_point,
            &primitives,
            textures,
        );
    }
}

/// A button in the top right corner, and under it the settings it opens.
fn layout(root: &mut egui::Ui, open: &mut bool, effects: &mut Effects, fps: f32) {
    let context = root.ctx().clone();
    egui::Area::new(egui::Id::new("settings button"))
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .show(&context, |ui| {
            if ui.selectable_label(*open, "Settings").clicked() {
                *open = !*open;
            }
        });
    if !*open {
        return;
    }
    egui::Window::new("Settings")
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 44.0])
        .title_bar(false)
        .resizable(false)
        .default_width(260.0)
        .show(&context, |ui| {
            ui.label(format!("{fps:.0} fps"));
            ui.separator();
            ui.checkbox(&mut effects.shadows, "Shadows");
            ui.checkbox(&mut effects.grass, "Grass");
            ui.checkbox(&mut effects.clouds, "Clouds");
            ui.add_enabled_ui(effects.clouds, |ui| {
                ui.add(egui::Slider::new(&mut effects.cloud_cover, 0.0..=1.0).text("Cover"));
                ui.add(egui::Slider::new(&mut effects.cloud_density, 0.1..=3.0).text("Density"));
                ui.add(egui::Slider::new(&mut effects.wind_m_s, 0.0..=80.0).text("Wind m/s"));
                ui.add(egui::Slider::new(&mut effects.cloud_change, 0.0..=6.0).text("Change"));
            });
            ui.separator();
            ui.add(egui::Slider::new(&mut effects.haze, 0.0..=6.0).text("Haze"));
            ui.add(
                egui::Slider::new(&mut effects.water_clarity, 0.25..=12.0).text("Water clarity"),
            );
            ui.add(egui::Slider::new(&mut effects.bloom, 0.0..=3.0).text("Bloom"));
            ui.add_enabled(
                effects.bloom > 0.0,
                egui::Slider::new(&mut effects.bloom_threshold, 0.2..=8.0).text("Bloom from"),
            );
            ui.add(egui::Slider::new(&mut effects.exposure, 0.1..=4.0).text("Exposure"));
            egui::ComboBox::from_label("Tone map")
                .selected_text(tone_map_name(effects.tone_map))
                .show_ui(ui, |ui| {
                    for each in ToneMap::ALL {
                        ui.selectable_value(&mut effects.tone_map, each, tone_map_name(each));
                    }
                });
            if ui.button("Defaults").clicked() {
                *effects = Effects::default();
            }
        });
}

/// A button in the bottom right corner that starts building, and while a
/// tool is in hand the tools and the paints instead. What was asked for.
fn build_layout(root: &mut egui::Ui, building: &Building) -> Option<Command> {
    let context = root.ctx().clone();
    let mut asked = None;
    let Some(tool) = building.tool else {
        egui::Area::new(egui::Id::new("build button"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-12.0, -12.0])
            .show(&context, |ui| {
                if ui.button("Build").clicked() {
                    asked = Some(Command::SetTool {
                        tool: Some(building.last),
                    });
                }
            });
        return asked;
    };
    egui::Window::new("Build")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-12.0, -12.0])
        .title_bar(false)
        .resizable(false)
        .default_width(260.0)
        .show(&context, |ui| {
            ui.horizontal(|ui| {
                for (each, name) in [
                    (Tool::Create, "Create"),
                    (Tool::Delete, "Delete"),
                    (Tool::Paint, "Paint"),
                ] {
                    if ui.selectable_label(tool == each, name).clicked() {
                        asked = Some(Command::SetTool { tool: Some(each) });
                    }
                }
                if ui.button("Close").clicked() {
                    asked = Some(Command::SetTool { tool: None });
                }
            });
            ui.horizontal(|ui| {
                let (undo, redo) = building.history;
                if ui.add_enabled(undo, egui::Button::new("Undo")).clicked() {
                    asked = Some(Command::Undo);
                }
                if ui.add_enabled(redo, egui::Button::new("Redo")).clicked() {
                    asked = Some(Command::Redo);
                }
            });
            ui.horizontal_wrapped(|ui| {
                for (index, &fill) in building.palette.iter().enumerate() {
                    let chosen = index == usize::from(building.paint);
                    let swatch = egui::Button::new("")
                        .fill(fill)
                        .min_size(egui::vec2(22.0, 22.0))
                        .stroke(egui::Stroke::new(
                            if chosen { 2.0 } else { 1.0 },
                            if chosen {
                                ui.visuals().strong_text_color()
                            } else {
                                ui.visuals().weak_text_color()
                            },
                        ));
                    if ui.add(swatch).clicked() {
                        asked = Some(Command::SetPaint { paint: index as u8 });
                    }
                }
            });
            ui.label(
                "Click and drag for a slab. Hold Alt as you start for a wall. \
                 Hold the right button to look around.",
            );
        });
    asked
}

/// `#rrggbb` as a colour.
fn color(hex: &str) -> Option<egui::Color32> {
    let digits = hex.strip_prefix('#')?;
    let byte = |at: usize| u8::from_str_radix(digits.get(at..at + 2)?, 16).ok();
    Some(egui::Color32::from_rgb(byte(0)?, byte(2)?, byte(4)?))
}

fn tone_map_name(tone_map: ToneMap) -> &'static str {
    match tone_map {
        ToneMap::Aces => "ACES",
        ToneMap::Agx => "AgX",
        ToneMap::Neutral => "Neutral",
        ToneMap::Reinhard => "Reinhard",
        ToneMap::Linear => "Linear",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: egui::Vec2 = egui::vec2(1280.0, 720.0);
    const WORLD: egui::Pos2 = egui::pos2(400.0, 400.0);
    /// On the settings button, in the top right corner.
    const BUTTON: egui::Pos2 = egui::pos2(1240.0, 22.0);

    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Whether the panel claims the pointer after these events, a frame each.
    fn claims(frames: &[&[egui::Event]]) -> bool {
        let context = egui::Context::default();
        let (mut open, mut effects) = (false, Effects::default());
        for (second, events) in frames.iter().enumerate() {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SCREEN)),
                time: Some(second as f64),
                events: events.to_vec(),
                ..Default::default()
            };
            let _ = context.run_ui(raw, |root| layout(root, &mut open, &mut effects, 60.0));
        }
        context.egui_wants_pointer_input()
    }

    #[test]
    fn a_click_on_the_world_does_not_cost_the_panel_its_next_click() {
        let settle: &[egui::Event] = &[];
        let capture = [egui::Event::PointerMoved(WORLD), press(WORLD, true)];
        let back = [egui::Event::PointerMoved(BUTTON)];
        // The shell once kept the release from the panel while the pointer was
        // the camera's: the press then read as a drag from outside, for ever.
        assert!(!claims(&[settle, &capture, &back, settle]));
        let release = [press(WORLD, false), egui::Event::PointerGone];
        assert!(claims(&[settle, &capture, &release, &back, settle]));
    }
}
