//! Headless render to PNG with a fixed clock and seed: how a change is looked
//! at without opening a window (CLAUDE.md, How it grows).

use client::Client;
use render::{Headless, write_png};

use crate::args::Shot;

pub fn run(shot: Shot) -> Result<(), String> {
    let [width, height] = shot.size;
    let mut client = Client::new(shot.recipe).map_err(|e| e.to_string())?;
    client.set_aspect(width as f32 / height as f32);
    client.set_clock(shot.clock_s);
    client.pose(shot.altitude_m, shot.pitch_deg.to_radians(), shot.boom_m);

    let frame = client.settled_frame();
    let mut headless = Headless::new(width, height)?;
    headless.renderer.apply(client.drain_terrain_changes());
    let pixels = headless.render(&frame);

    if let Some(parent) = shot.out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    write_png(&shot.out, width, height, &pixels)?;
    log::info!("{} patches -> {}", frame.patches.len(), shot.out.display());
    Ok(())
}
