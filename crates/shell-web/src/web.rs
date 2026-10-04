use std::cell::RefCell;
use std::rc::Rc;

use client::{Chord, Client, Input, Key, Outbound};
use render::{Gpu, Renderer, View, surface_configuration, wgpu};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, KeyboardEvent, MessageEvent, PointerEvent, WheelEvent};

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
    /// The socket to a world server, while the page wants one.
    link: Option<Link>,
    /// Building, the right button is held: the pointer turns the camera.
    looking: bool,
}

/// Where the web app serves the asset root.
const ASSET_ROOT: &str = "/assets/";

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
        let mut client =
            Client::new(client::Recipe::new(1)).map_err(|e| JsError::new(&e.to_string()))?;
        // What this version carries, off until a world says which are on.
        for plugin in plugins_client::all() {
            client.plug(plugin);
        }
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
            link: None,
            looking: false,
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

    /// The ground a field world is shaped by. Sent before the recipe that
    /// names it: a world cannot be generated without the field it was made
    /// from, so the page fetches it first.
    pub fn set_field(&self, bytes: Vec<u8>) -> Result<(), JsError> {
        self.state
            .borrow_mut()
            .client
            .set_field(bytes)
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Opens the link to a world server at a socket URL, the ticket already
    /// in it. The client speaks the protocol; this only carries frames. The
    /// page hears `session` events and decides when to connect again.
    pub fn connect(&self, url: &str) -> Result<(), JsError> {
        self.disconnect();
        let link = Link::open(url, &self.state).map_err(|e| JsError::new(&e))?;
        self.state.borrow_mut().link = Some(link);
        Ok(())
    }

    /// Closes the link, if one is open.
    pub fn disconnect(&self) {
        let link = self.state.borrow_mut().link.take();
        if let Some(link) = link {
            link.close();
            self.state.borrow_mut().client.link_closed();
        }
    }
}

/// One WebSocket, and its listeners, for as long as the page wants it.
struct Link {
    socket: web_sys::WebSocket,
    listeners: Vec<Listener>,
}

impl Link {
    fn open(url: &str, state: &Rc<RefCell<State>>) -> Result<Link, String> {
        let socket = web_sys::WebSocket::new(url).map_err(|e| format!("{url}: {e:?}"))?;
        socket.set_binary_type(web_sys::BinaryType::Arraybuffer);
        let target: &web_sys::EventTarget = socket.as_ref();

        let opened = {
            let state = state.clone();
            move |_: web_sys::Event| state.borrow_mut().client.link_opened()
        };
        let message = {
            let state = state.clone();
            move |event: web_sys::Event| {
                let event: MessageEvent = event.unchecked_into();
                if let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let frame = js_sys::Uint8Array::new(&buffer).to_vec();
                    state.borrow_mut().client.receive(&frame);
                }
            }
        };
        // A close after an error, or an error after a close: the client
        // hears one closing either way, and a second is harmless.
        let closed = {
            let state = state.clone();
            move |_: web_sys::Event| {
                let mut state = state.borrow_mut();
                if state.link.is_some() {
                    state.link = None;
                    state.client.link_closed();
                }
            }
        };
        let listeners = vec![
            Listener::add(target, "open", opened),
            Listener::add(target, "message", message),
            Listener::add(target, "close", closed.clone()),
            Listener::add(target, "error", closed),
        ];
        Ok(Link { socket, listeners })
    }

    fn send(&self, outbound: Outbound) {
        match outbound {
            Outbound::Frame(frame) => {
                if let Err(error) = self.socket.send_with_u8_array(&frame) {
                    log::warn!("send: {error:?}");
                }
            }
            Outbound::Close => self.socket.close().unwrap_or(()),
        }
    }

    fn close(self) {
        for listener in self.listeners {
            listener.remove();
        }
        let _ = self.socket.close();
    }
}

/// Sends what the client queued for the link.
fn pump_link(state: &Rc<RefCell<State>>) {
    let mut held = state.borrow_mut();
    let outbound = held.client.drain_outbound();
    if let Some(link) = &held.link {
        for out in outbound {
            link.send(out);
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.disconnect();
        *self.alive.borrow_mut() = false;
        for listener in self.listeners.drain(..) {
            listener.remove();
        }
        if let Some(document) = web_sys::window().and_then(|w| w.document()) {
            document.exit_pointer_lock();
        }
    }
}

/// The browser side of the asset seam: one `fetch` per request, answered
/// whenever it lands. A dropped engine simply never hears the answer.
fn fetch_assets(state: &Rc<RefCell<State>>) {
    for request in state.borrow_mut().client.drain_asset_requests() {
        let state = Rc::downgrade(state);
        wasm_bindgen_futures::spawn_local(async move {
            let bytes = fetch(&resolve(&request.path)).await;
            if let Some(state) = state.upgrade() {
                state.borrow_mut().client.asset_loaded(request.id, bytes);
            }
        });
    }
}

/// An asset reference as a URL: absolute ones (a user's own avatar) pass
/// through, relative ones live under the asset root.
fn resolve(reference: &str) -> String {
    let absolute = reference.starts_with('/') || reference.contains("://");
    if absolute {
        reference.to_owned()
    } else {
        format!("{ASSET_ROOT}{reference}")
    }
}

async fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let describe = |error: JsValue| format!("{url}: {error:?}");
    let window = web_sys::window().ok_or("no window")?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(describe)?;
    let response: web_sys::Response = response.unchecked_into();
    if !response.ok() {
        return Err(format!("{url}: HTTP {}", response.status()));
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer().map_err(describe)?)
        .await
        .map_err(describe)?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

impl State {
    /// One frame. Returns the events it produced rather than handing them to
    /// JS here: the whole of this runs inside a `RefCell` borrow of the state,
    /// and a handler that answers an event with a command, which is the most
    /// natural thing a front end does, would re-enter that borrow and panic.
    /// The caller dispatches them once the borrow is gone.
    #[must_use]
    fn frame(&mut self) -> Vec<JsValue> {
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
        // A tool aims with a free pointer: taking one lets a captured one go.
        if self.client.pointing() && locked(&self.canvas) {
            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                document.exit_pointer_lock();
            }
        }
        self.renderer.apply(self.client.drain_terrain_changes());
        self.renderer
            .apply_volumes(self.client.drain_volume_changes());
        self.renderer
            .apply_skinned(self.client.drain_skinned_changes());
        let events: Vec<JsValue> = self
            .client
            .drain_events()
            .iter()
            .map(|event| JsValue::from_str(&event.to_json()))
            .collect();

        use wgpu::CurrentSurfaceTexture::{Lost, Outdated, Suboptimal, Success};
        let texture = match self.surface.get_current_texture() {
            Success(texture) | Suboptimal(texture) => texture,
            // No picture this frame, but the world still moved and still has
            // things to say, so the events go out either way.
            Outdated | Lost => {
                self.surface.configure(&self.gpu.device, &self.config);
                return events;
            }
            _ => return events,
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
        events
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
        fetch_assets(&state);
        pump_link(&state);
        let (events, on_event) = {
            let mut held = state.borrow_mut();
            let events = held.frame();
            (events, held.on_event.clone())
        };
        // Outside the borrow: a handler is free to answer with a command.
        for json in events {
            if let Err(error) = on_event.call1(&JsValue::NULL, &json) {
                log::error!("event handler threw: {error:?}");
            }
        }
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

/// Whether the pointer is captured by this canvas.
fn locked(canvas: &HtmlCanvasElement) -> bool {
    let locked = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.pointer_lock_element());
    let own: &web_sys::Element = canvas.as_ref();
    locked.is_some_and(|element| &element == own)
}

/// Where a pointer event is over the canvas, as fractions from the top left.
fn fraction(canvas: &HtmlCanvasElement, event: &PointerEvent) -> [f32; 2] {
    [
        event.offset_x() as f32 / canvas.client_width().max(1) as f32,
        event.offset_y() as f32 / canvas.client_height().max(1) as f32,
    ]
}

/// Pointer buttons as the DOM numbers them.
const PRIMARY: i16 = 0;
const SECONDARY: i16 = 2;

/// Whether the page has the keys for something typed into: a field of the
/// panels, which the canvas leaves them to.
fn typing() -> bool {
    let focused = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.active_element());
    focused.is_some_and(|element| {
        let editable = element.has_attribute("contenteditable");
        editable || matches!(element.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
    })
}

fn listen(canvas: &HtmlCanvasElement, state: &Rc<RefCell<State>>) -> Vec<Listener> {
    let target: &web_sys::EventTarget = canvas.as_ref();

    let key = |down: bool| {
        let state = state.clone();
        move |event: web_sys::Event| {
            let event: KeyboardEvent = event.unchecked_into();
            let code = event.code();
            // Command on a Mac, Control elsewhere.
            let chord = Chord {
                command: event.meta_key() || event.ctrl_key(),
                shift: event.shift_key(),
            };
            let mut state = state.borrow_mut();
            if !down {
                state.input.hold(&code, false);
            }
            if state.client.asks(&code, chord) {
                // A key a plugin asked for is the plugin's, and the page has
                // nothing of its own to do with it here.
                event.prevent_default();
                if down && !event.repeat() {
                    state.input.code(&code, chord, true);
                }
            } else if let Some(key) = binding(&code) {
                event.prevent_default();
                // One shot keys must not fire again while held.
                if !(down && event.repeat()) {
                    state.input.key(key, down);
                }
            }
        }
    };
    // While a plugin has the pointer, the primary button is its tool's and
    // the secondary one looks while it is held, so the pointer stays free to
    // aim. Otherwise a press captures the pointer, and the camera has it
    // until Escape.
    let press = {
        let (state, canvas) = (state.clone(), canvas.clone());
        move |event: web_sys::Event| {
            let event: PointerEvent = event.unchecked_into();
            let _ = canvas.focus();
            let mut state = state.borrow_mut();
            if state.client.pointing() {
                event.prevent_default();
                let _ = canvas.set_pointer_capture(event.pointer_id());
                state.input.pointer = Some(fraction(&canvas, &event));
                // The pointer says what is held with it, whoever had the
                // keys when it went down.
                hold_alt(&mut state.input, event.alt_key());
                match event.button() {
                    PRIMARY => state.input.key(Key::Use, true),
                    SECONDARY => state.looking = true,
                    _ => {}
                }
            } else if event.button() == PRIMARY {
                canvas.request_pointer_lock();
            }
        }
    };
    let release = {
        let state = state.clone();
        move |event: web_sys::Event| {
            let event: PointerEvent = event.unchecked_into();
            let mut state = state.borrow_mut();
            match event.button() {
                PRIMARY => state.input.key(Key::Use, false),
                SECONDARY => state.looking = false,
                _ => {}
            }
        }
    };
    let motion = {
        let (state, canvas) = (state.clone(), canvas.clone());
        move |event: web_sys::Event| {
            let event: PointerEvent = event.unchecked_into();
            let mut state = state.borrow_mut();
            let turning = locked(&canvas) || state.looking;
            if !locked(&canvas) {
                state.input.pointer = Some(fraction(&canvas, &event));
            }
            // With a tool in hand, the keys are the tool's while the pointer
            // is over the world: a button of the panel that was clicked last
            // keeps them otherwise, and a key meant for the stroke is lost.
            if state.client.pointing() {
                hold_alt(&mut state.input, event.alt_key());
                if !typing() {
                    let _ = canvas.focus();
                }
            }
            if turning {
                state.input.look[0] += event.movement_x() as f32;
                state.input.look[1] += event.movement_y() as f32;
            }
        }
    };
    // The secondary button looks while a tool is in hand; the page keeps
    // its menu.
    let menu = {
        let state = state.clone();
        move |event: web_sys::Event| {
            if state.borrow().client.pointing() {
                event.prevent_default();
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
        move |_: web_sys::Event| {
            let mut state = state.borrow_mut();
            state.input.release_all();
            state.looking = false;
        }
    };

    vec![
        Listener::add(target, "keydown", key(true)),
        Listener::add(target, "keyup", key(false)),
        Listener::add(target, "pointerdown", press),
        Listener::add(target, "pointerup", release),
        Listener::add(target, "pointermove", motion),
        Listener::add(target, "contextmenu", menu),
        Listener::add(target, "wheel", wheel),
        Listener::add(target, "blur", blur),
    ]
}

/// Keeps Alt in step with what a pointer event says of it. A plugin asks for
/// either Alt by the name the web gives it.
fn hold_alt(input: &mut Input, down: bool) {
    input.hold("AltLeft", down);
    if !down {
        input.hold("AltRight", false);
    }
}

/// The core's key bindings. Mirrors `shell-desktop`, with DOM
/// `KeyboardEvent.code` names. A plugin's keys are asked for by name
/// ([`client::Client::asks`]) and reach it by the same names.
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
        "KeyV" => Key::NextAvatar,
        _ => return None,
    })
}
