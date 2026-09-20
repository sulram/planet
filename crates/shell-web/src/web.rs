use std::cell::RefCell;
use std::rc::Rc;

use client::{Client, Input, Key};
use render::{Gpu, Renderer, View, surface_configuration, wgpu};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
}

/// The engine on one canvas. It owns its animation loop and its input
/// listeners; dropping it (`free()` in JS) stops both.
#[wasm_bindgen]
pub struct Engine {
    state: Rc<RefCell<State>>,
    /// Cleared on drop so the loop ends and listeners go quiet.
    alive: Rc<RefCell<bool>>,
    listeners: Vec<Listener>,
}

struct State {
    canvas: HtmlCanvasElement,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: Gpu,
    renderer: Renderer,
    client: Client,
    input: Input,
    on_event: js_sys::Function,
    last_frame_ms: f64,
}

#[wasm_bindgen]
impl Engine {
    /// Rejects when the browser offers neither WebGPU nor WebGL2.
    pub async fn create(
        canvas: HtmlCanvasElement,
        on_event: js_sys::Function,
    ) -> Result<Engine, JsError> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsError::new(&e.to_string()))?;
        let gpu = Gpu::new(&instance, Some(&surface))
            .await
            .map_err(|e| JsError::new(&e))?;
        let config = surface_configuration(&surface, &gpu.adapter, canvas.width(), canvas.height());
        surface.configure(&gpu.device, &config);
        let renderer = Renderer::new(&gpu, config.format);

        // A placeholder world until the UI sends the recipe it wants.
        let client =
            Client::new(client::Recipe::new(1)).map_err(|e| JsError::new(&e.to_string()))?;
        let state = Rc::new(RefCell::new(State {
            canvas: canvas.clone(),
            surface,
            config,
            gpu,
            renderer,
            client,
            input: Input::default(),
            on_event,
            last_frame_ms: now_ms(),
        }));
        let alive = Rc::new(RefCell::new(true));
        let listeners = listen(&canvas, &state);
        run(state.clone(), alive.clone());
        Ok(Engine {
            state,
            alive,
            listeners,
        })
    }

    /// One `client::Command` as JSON.
    pub fn command(&self, json: &str) {
        self.state.borrow_mut().client.command_json(json);
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        *self.alive.borrow_mut() = false;
        for listener in self.listeners.drain(..) {
            listener.remove();
        }
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            document.exit_pointer_lock();
        }
    }
}

impl State {
    fn frame(&mut self) {
        let now = now_ms();
        let dt = (now - self.last_frame_ms) / 1000.0;
        self.last_frame_ms = now;

        // The canvas follows its CSS box; the backing store follows the canvas.
        let ratio = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
        let width = ((f64::from(self.canvas.client_width()) * ratio) as u32).max(1);
        let height = ((f64::from(self.canvas.client_height()) * ratio) as u32).max(1);
        if (width, height) != (self.config.width, self.config.height) {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.gpu.device, &self.config);
        }

        self.client.set_aspect(width as f32 / height as f32);
        let frame = self.client.update(dt, &mut self.input);
        self.renderer.apply(self.client.drain_terrain_changes());
        for event in self.client.drain_events() {
            let json = JsValue::from_str(&event.to_json());
            if let Err(error) = self.on_event.call1(&JsValue::NULL, &json) {
                log::error!("event handler threw: {error:?}");
            }
        }

        use wgpu::CurrentSurfaceTexture::{Lost, Outdated, Suboptimal, Success};
        let texture = match self.surface.get_current_texture() {
            Success(texture) | Suboptimal(texture) => texture,
            Outdated | Lost => return self.surface.configure(&self.gpu.device, &self.config),
            _ => return,
        };
        let target = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let view = View {
            camera: frame.camera,
            target: &target,
            size: [width, height],
        };
        self.renderer.render(&frame, &[view]);
        self.gpu.queue.present(texture);
    }
}

/// The loop closure holds a handle to itself so it can reschedule.
type Tick = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// The animation loop: one closure that reschedules itself while `alive`.
fn run(state: Rc<RefCell<State>>, alive: Rc<RefCell<bool>>) {
    let tick: Tick = Rc::new(RefCell::new(None));
    let again = tick.clone();
    *tick.borrow_mut() = Some(Closure::new(move || {
        if !*alive.borrow() {
            // Dropping the closure here breaks the cycle that kept it alive.
            again.borrow_mut().take();
            return;
        }
        state.borrow_mut().frame();
        request_frame(again.borrow().as_ref().expect("the loop closure"));
    }));
    request_frame(tick.borrow().as_ref().expect("the loop closure"));
}

fn request_frame(tick: &Closure<dyn FnMut()>) {
    let window = web_sys::window().expect("a window");
    window
        .request_animation_frame(tick.as_ref().unchecked_ref())
        .expect("requestAnimationFrame");
}

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map_or(0.0, |p| p.now())
}

/// A DOM listener that can be removed again.
struct Listener {
    target: web_sys::EventTarget,
    kind: &'static str,
    closure: Closure<dyn FnMut(web_sys::Event)>,
}

impl Listener {
    fn add(
        target: &web_sys::EventTarget,
        kind: &'static str,
        handler: impl FnMut(web_sys::Event) + 'static,
    ) -> Listener {
        let closure = Closure::new(handler);
        // Not passive: the wheel handler must be able to stop the page scroll.
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(false);
        target
            .add_event_listener_with_callback_and_add_event_listener_options(
                kind,
                closure.as_ref().unchecked_ref(),
                &options,
            )
            .expect("addEventListener");
        Listener {
            target: target.clone(),
            kind,
            closure,
        }
    }

    fn remove(self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.kind, self.closure.as_ref().unchecked_ref());
    }
}

fn listen(canvas: &HtmlCanvasElement, state: &Rc<RefCell<State>>) -> Vec<Listener> {
    let target: &web_sys::EventTarget = canvas.as_ref();
    let is_locked = {
        let canvas = canvas.clone();
        move || {
            let locked = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.pointer_lock_element());
            let own: &web_sys::Element = canvas.as_ref();
            locked.is_some_and(|element| &element == own)
        }
    };

    let key = |down: bool| {
        let state = state.clone();
        move |event: web_sys::Event| {
            let event: KeyboardEvent = event.unchecked_into();
            if let Some(key) = binding(&event.code()) {
                event.prevent_default();
                // One shot keys must not fire again while held.
                if !(down && event.repeat()) {
                    state.borrow_mut().input.key(key, down);
                }
            }
        }
    };
    let click = {
        let canvas = canvas.clone();
        move |_: web_sys::Event| {
            let _ = canvas.focus();
            canvas.request_pointer_lock();
        }
    };
    let motion = {
        let (state, is_locked) = (state.clone(), is_locked.clone());
        move |event: web_sys::Event| {
            if is_locked() {
                let event: MouseEvent = event.unchecked_into();
                let mut state = state.borrow_mut();
                state.input.look[0] += event.movement_x() as f32;
                state.input.look[1] += event.movement_y() as f32;
            }
        }
    };
    let wheel = {
        let state = state.clone();
        move |event: web_sys::Event| {
            event.prevent_default();
            let event: WheelEvent = event.unchecked_into();
            state.borrow_mut().input.zoom -= (event.delta_y() / 100.0) as f32;
        }
    };
    let blur = {
        let state = state.clone();
        move |_: web_sys::Event| state.borrow_mut().input.release_all()
    };

    vec![
        Listener::add(target, "keydown", key(true)),
        Listener::add(target, "keyup", key(false)),
        Listener::add(target, "click", click),
        Listener::add(target, "mousemove", motion),
        Listener::add(target, "wheel", wheel),
        Listener::add(target, "blur", blur),
    ]
}

/// Key bindings. Mirrors `shell-desktop`, with DOM `KeyboardEvent.code` names.
fn binding(code: &str) -> Option<Key> {
    Some(match code {
        "KeyW" | "ArrowUp" => Key::Forward,
        "KeyS" | "ArrowDown" => Key::Back,
        "KeyA" | "ArrowLeft" => Key::Left,
        "KeyD" | "ArrowRight" => Key::Right,
        "Space" => Key::Up,
        "KeyC" | "ControlLeft" => Key::Down,
        "ShiftLeft" | "ShiftRight" => Key::Sprint,
        "KeyF" => Key::ToggleMode,
        "KeyR" => Key::NewSeed,
        _ => return None,
    })
}
