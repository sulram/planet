//! The browser shell. What touches the browser lives behind `cfg(wasm32)`, so
//! `cargo test --workspace` stays native; the fingers are plain logic, built
//! everywhere so their tests run there.
//!
//! The JS side sees one class, [`web::Engine`], and talks to it through the
//! command/event seam as JSON (`client::Command`, `client::Event`), plus the
//! keys it presses and the stick the shell itself says.

#[cfg(target_arch = "wasm32")]
mod audio;
#[cfg(target_arch = "wasm32")]
mod moving;
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
mod touch;
#[cfg(target_arch = "wasm32")]
mod web;
