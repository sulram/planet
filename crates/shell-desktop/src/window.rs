//! The windowed explorer.

use std::sync::Arc;
use std::time::Instant;

use client::{Client, Event, Field, Input, Key};

use crate::assets;
use render::{Gpu, Renderer, View, surface_configuration, wgpu};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{
    DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};
use worldgen::{Recipe, format_seed};

pub fn run(
    recipe: Recipe,
    field: Option<Field>,
    avatar: Option<String>,
    at: Option<String>,
) -> Result<(), String> {
    let mut client = match field {
        Some(field) => Client::with_field(recipe, field),
        None => Client::new(recipe),
    }
    .map_err(|e| e.to_string())?;
    if let Some(place) = &at {
        client.go_to(place).map_err(|e| format!("--at {e}"))?;
    }
    let since_epoch = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    client.add_entropy(since_epoch.map_or(0, |elapsed| elapsed.as_nanos() as u64));
    assets::wear(&mut client, avatar.as_deref());
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    let mut app = App {
        client,
        input: Input::default(),
        modifiers: ModifiersState::empty(),
        stage: None,
        failure: None,
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    app.failure.map_or(Ok(()), Err)
}

struct App {
    client: Client,
    input: Input,
    /// Held modifiers, for the keys that take one: undo and redo.
    modifiers: ModifiersState,
    /// Exists between `resumed` and exit.
    stage: Option<Stage>,
    failure: Option<String>,
}

/// The window and everything that draws into it.
struct Stage {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    renderer: Renderer,
    panel: ui_native::Panel,
    last_frame: Instant,
    /// The pointer is captured and steers the camera.
    looking: bool,
}

impl Stage {
    fn new(event_loop: &ActiveEventLoop) -> Result<Stage, String> {
        let attributes = Window::default_attributes()
            .with_title("planet")
            .with_inner_size(PhysicalSize::new(1280, 720));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|e| e.to_string())?,
        );
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
        let gpu = pollster::block_on(Gpu::new(&instance, Some(&surface)))?;

        let size = window.inner_size();
        let config = surface_configuration(&surface, &gpu.adapter, size.width, size.height);
        surface.configure(&gpu.device, &config);
        let renderer = Renderer::new(&gpu, config.format);
        let panel = ui_native::Panel::new(&gpu.device, config.format, &window);
        Ok(Stage {
            window,
            surface,
            config,
            gpu,
            renderer,
            panel,
            last_frame: Instant::now(),
            looking: false,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.gpu.device, &self.config);
    }

    fn set_looking(&mut self, looking: bool) {
        if looking {
            // Locked where the platform has it (macOS, Wayland), confined elsewhere.
            let grabbed = self
                .window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));
            if grabbed.is_err() {
                return;
            }
        } else {
            let _ = self.window.set_cursor_grab(CursorGrabMode::None);
        }
        self.window.set_cursor_visible(!looking);
        self.looking = looking;
        if looking {
            self.panel.pointer_gone();
        }
    }

    fn draw(&mut self, client: &mut Client, input: &mut Input) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f64();
        self.last_frame = now;

        client.set_aspect(self.config.width as f32 / self.config.height as f32);
        assets::serve(client);
        let frame = client.update(dt, input);
        self.renderer.apply(client.drain_terrain_changes());
        self.renderer.apply_volumes(client.drain_volume_changes());
        self.renderer.apply_skinned(client.drain_skinned_changes());
        for event in client.drain_events() {
            self.panel.event(&event);
            match event {
                // The title bar is the native HUD until there is a native UI.
                Event::Stats {
                    fps,
                    altitude_m,
                    ref place,
                    ..
                } => {
                    let seed = format_seed(client.recipe().seed);
                    self.window.set_title(&format!(
                        "planet {seed} | {place} | {altitude_m:.0} m | {fps:.0} fps"
                    ));
                }
                Event::AvatarChanged { path } => log::info!("avatar: {path}"),
                // A tool aims with a free pointer: taking one lets it go.
                Event::ToolChanged { tool: Some(_), .. } => self.set_looking(false),
                Event::BuildRefused { reason } => log::info!("no volume here: {reason:?}"),
                // The desktop has no socket yet (ROADMAP M2): the link stays
                // offline and nobody else is ever here.
                Event::RecipeChanged { .. }
                | Event::Ready { .. }
                | Event::ModeChanged { .. }
                | Event::EffectsChanged { .. }
                | Event::Session { .. }
                | Event::Peers { .. }
                | Event::Said { .. }
                | Event::Anchors { .. }
                | Event::ToolChanged { .. }
                | Event::Palette { .. }
                | Event::History { .. }
                | Event::Settled => {}
                Event::Rejected { message } => log::warn!("command rejected: {message}"),
            }
        }

        use wgpu::CurrentSurfaceTexture::{
            Lost, Occluded, Outdated, Suboptimal, Success, Timeout, Validation,
        };
        let texture = match self.surface.get_current_texture() {
            Success(texture) | Suboptimal(texture) => texture,
            Outdated | Lost => {
                self.surface.configure(&self.gpu.device, &self.config);
                return;
            }
            Timeout | Occluded | Validation => return,
        };
        let target = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let size = [self.config.width, self.config.height];
        self.renderer.render(
            &frame,
            &[View {
                camera: frame.camera,
                target: &target,
                size,
            }],
        );
        // The panel goes over the finished picture, in a submission of its own.
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("panel"),
            });
        let commands = self.panel.frame(
            &self.window,
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &target,
            size,
        );
        self.gpu.queue.submit([encoder.finish()]);
        for command in commands {
            client.command(command);
        }
        self.gpu.queue.present(texture);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.stage.is_some() {
            return;
        }
        match Stage::new(event_loop) {
            Ok(stage) => {
                stage.window.request_redraw();
                self.stage = Some(stage);
            }
            Err(message) => {
                self.failure = Some(message);
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(stage) = &mut self.stage else { return };
        // The panel first, while the pointer is free: a click on a slider is
        // not a click on the world. Captured, the pointer is the camera's, but
        // the panel still hears buttons let go: the click that captured it was
        // pressed in the panel's sight, and a press it never sees released
        // reads as a drag from outside, which makes it refuse every click.
        // A release reaches the world too, wherever it lands: a stroke
        // dragged onto the panel ends when the button does.
        let released = matches!(
            event,
            WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            }
        );
        if !released
            && !stage.looking
            && stage.panel.window_event(&stage.window, &event)
        {
            return;
        }
        if released {
            stage.panel.window_event(&stage.window, &event);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => stage.resize(size),
            WindowEvent::Focused(false) => {
                self.input.release_all();
                stage.set_looking(false);
            }
            // Building, the left button is the tool's and the right one looks
            // while it is held. Otherwise a click captures the pointer, and
            // the camera has it until Escape.
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                match (self.client.building(), button) {
                    (true, MouseButton::Left) => self.input.key(Key::Use, down),
                    (true, MouseButton::Right) => stage.set_looking(down),
                    (false, MouseButton::Left) if down => stage.set_looking(true),
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } if !stage.looking => {
                let size = stage.window.inner_size();
                self.input.pointer = Some([
                    position.x as f32 / size.width.max(1) as f32,
                    position.y as f32 / size.height.max(1) as f32,
                ]);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.input.zoom += match delta {
                    MouseScrollDelta::LineDelta(_, lines) => lines,
                    MouseScrollDelta::PixelDelta(pixels) => pixels.y as f32 / 40.0,
                };
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                let down = event.state == ElementState::Pressed;
                // Command on a Mac, Control elsewhere.
                let command = self.modifiers.super_key() || self.modifiers.control_key();
                let undo = match code {
                    KeyCode::KeyZ if command && self.modifiers.shift_key() => Some(Key::Redo),
                    KeyCode::KeyZ if command => Some(Key::Undo),
                    KeyCode::KeyY if command => Some(Key::Redo),
                    _ => None,
                };
                if let Some(key) = undo {
                    if down && !event.repeat {
                        self.input.key(key, true);
                    }
                } else if code == KeyCode::Escape && down {
                    // A captured pointer goes back first; building, Escape
                    // drops the stroke, then the tool.
                    if stage.looking {
                        stage.set_looking(false);
                    } else {
                        self.input.key(Key::Cancel, true);
                    }
                } else if let Some(key) = binding(code) {
                    // One shot keys must not fire again while held.
                    if !(down && event.repeat) {
                        self.input.key(key, down);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                stage.draw(&mut self.client, &mut self.input);
                stage.window.request_redraw();
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        let looking = self.stage.as_ref().is_some_and(|stage| stage.looking);
        if let (true, DeviceEvent::MouseMotion { delta }) = (looking, event) {
            self.input.look[0] += delta.0 as f32;
            self.input.look[1] += delta.1 as f32;
        }
    }
}

/// Key bindings. `shell-web` mirrors this table with DOM key codes.
fn binding(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::KeyW | KeyCode::ArrowUp => Key::Forward,
        KeyCode::KeyS | KeyCode::ArrowDown => Key::Back,
        KeyCode::KeyA | KeyCode::ArrowLeft => Key::Left,
        KeyCode::KeyD | KeyCode::ArrowRight => Key::Right,
        KeyCode::Space => Key::Up,
        KeyCode::KeyC | KeyCode::ControlLeft => Key::Down,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Sprint,
        KeyCode::KeyF => Key::ToggleMode,
        KeyCode::KeyR => Key::NewSeed,
        KeyCode::KeyV => Key::NextAvatar,
        KeyCode::KeyB => Key::Build,
        KeyCode::Digit1 => Key::Create,
        KeyCode::Digit2 => Key::Delete,
        KeyCode::Digit3 => Key::Paint,
        KeyCode::AltLeft | KeyCode::AltRight => Key::Upright,
        _ => return None,
    })
}
