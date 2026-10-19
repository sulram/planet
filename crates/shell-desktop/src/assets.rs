//! The desktop side of the asset seam: files under one root folder, and a
//! picture decoded here, so the engine carries no codec.

use std::path::PathBuf;

use client::{Client, Command, Form};

/// `PLANET_ASSETS`, else `./assets`: run from the repo root, or point at a copy.
fn root() -> PathBuf {
    std::env::var_os("PLANET_ASSETS").map_or_else(|| PathBuf::from("assets"), PathBuf::from)
}

/// Where a world's own files are read from, `asset://<hash>.<ext>` by name:
/// `PLANET_FILES`, the `files/` folder of a world folder. Unset, a world's
/// file is out of reach until the desktop opens the socket (ROADMAP, Other
/// screens).
fn files() -> Option<PathBuf> {
    std::env::var_os("PLANET_FILES").map(PathBuf::from)
}

/// Answers every pending request from disk. Reads are small and local, so
/// they happen inline.
pub fn serve(client: &mut Client) {
    for request in client.drain_asset_requests() {
        let path = if let Some(name) = request.path.strip_prefix("asset://") {
            files().map(|files| files.join(name))
        } else if request.path.contains("://") {
            // An absolute URL needs an HTTP client, which arrives with the
            // socket.
            None
        } else {
            Some(root().join(&request.path))
        };
        let bytes = match path {
            Some(path) => std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display())),
            None => Err(format!("{}: out of the desktop's reach", request.path)),
        };
        match request.form {
            Form::Bytes => client.asset_loaded(request.id, bytes),
            Form::Picture { max_side } => {
                client.picture_loaded(request.id, bytes.and_then(|bytes| decode(&bytes, max_side)))
            }
        }
    }
}

/// A picture's bytes as RGBA pixels, no wider than `max_side` on a side.
fn decode(bytes: &[u8], max_side: u32) -> Result<scene::Image, String> {
    let decoded = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let decoded = if decoded.width().max(decoded.height()) > max_side {
        decoded.resize(max_side, max_side, image::imageops::FilterType::Triangle)
    } else {
        decoded
    };
    let rgba = decoded.into_rgba8();
    Ok(scene::Image {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
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
