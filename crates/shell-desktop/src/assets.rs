//! The desktop side of the asset seam: files under one root folder.

use std::path::PathBuf;

use client::{Client, Command};

/// `PLANET_ASSETS`, else `./assets`: run from the repo root, or point at a copy.
fn root() -> PathBuf {
    std::env::var_os("PLANET_ASSETS").map_or_else(|| PathBuf::from("assets"), PathBuf::from)
}

/// Answers every pending request from disk. Reads are small and local, so
/// they happen inline.
pub fn serve(client: &mut Client) {
    for request in client.drain_asset_requests() {
        // Absolute URLs (a user's own avatar) need an HTTP client, which
        // arrives with login on native (ROADMAP M2).
        let bytes = if request.path.contains("://") {
            Err(format!(
                "{}: remote assets are not supported on desktop yet",
                request.path
            ))
        } else {
            let path = root().join(&request.path);
            std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        client.asset_loaded(request.id, bytes);
    }
}

/// Wears the named avatar (`Kyle`), or any avatar the manifest offers.
pub fn wear(client: &mut Client, name: Option<&str>) {
    client.command(match name {
        Some(name) => Command::SetAvatar {
            path: format!("avatars/{name}.vrm"),
        },
        None => Command::RandomAvatar,
    });
}
