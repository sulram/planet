//! The browser shell. Everything lives behind `cfg(wasm32)`: on other targets
//! this crate is empty, so `cargo test --workspace` stays native.
//!
//! The JS side sees one class, [`web::Engine`], and talks to it only through
//! the command/event seam as JSON (`client::Command`, `client::Event`).

#[cfg(target_arch = "wasm32")]
mod web;
