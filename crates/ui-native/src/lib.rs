//! The native settings panel. A front-end like the Svelte one: it renders
//! [`Event`]s and answers with [`Command`]s, and knows nothing else of the
//! world. All of egui lives in this crate; a shell hands it window events, a
//! place to paint, and takes the commands back.

mod paint;

use client::{Command, Effects, Event};
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

    /// What the client said this frame.
    pub fn event(&mut self, event: &Event) {
        match event {
            Event::EffectsChanged { effects } => self.effects = *effects,
            Event::Stats { fps, .. } => self.fps = *fps,
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
        let output = self
            .context
            .run_ui(raw, |root| layout(root, &mut open, &mut effects, fps));
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

        if effects == self.effects {
            return Vec::new();
        }
        // Shown at once; the client's answer, clamped, replaces it next frame.
        self.effects = effects;
        vec![Command::SetEffects { effects }]
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
            ui.add(egui::Slider::new(&mut effects.exposure, 0.1..=4.0).text("Exposure"));
            if ui.button("Defaults").clicked() {
                *effects = Effects::default();
            }
        });
}
