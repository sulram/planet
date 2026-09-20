//! Headless render to PNG with a fixed clock and seed: how a change is looked
//! at without opening a window (CLAUDE.md, How it grows).

use client::Client;
use render::{Headless, write_png};

use crate::args::Shot;
use crate::assets;

pub fn run(shot: Shot) -> Result<(), String> {
    let [width, height] = shot.size;
    let mut client = match shot.field {
        Some(field) => Client::with_field(shot.recipe, field),
        None => Client::new(shot.recipe),
    }
    .map_err(|e| e.to_string())?;
    client.set_aspect(width as f32 / height as f32);
    client.set_clock(shot.clock_s);
    if let Some(effects) = &shot.effects {
        client.command_json(&format!(r#"{{"type":"set_effects","effects":{effects}}}"#));
    }
    if let Some([u, v]) = shot.at {
        client.teleport(u, v);
    }
    let (pitch, boom_m) = (shot.pitch_deg.to_radians(), shot.boom_m);
    match shot.moon_gap_m {
        Some(gap_m) => client.visit_moon(gap_m, pitch, boom_m),
        None => client.pose(shot.altitude_m, pitch, boom_m),
    }
    assets::wear(&mut client, shot.avatar.as_deref());
    // Manifest first, then what it names: two rounds.
    assets::serve(&mut client);
    assets::serve(&mut client);
    for event in client.drain_events() {
        if let client::Event::Rejected { message } = event {
            log::warn!("{message}");
        }
    }

    // A fixed step keeps the stride, and so the picture, reproducible.
    let mut input = client::Input::default();
    input.key(client::Key::Forward, true);
    for _ in 0..(shot.walk_s * 60.0) as u32 {
        client.update(1.0 / 60.0, &mut input);
    }
    client.set_clock(shot.clock_s);

    let frame = client.settled_frame();
    let mut headless = Headless::new(width, height)?;
    headless.renderer.apply(client.drain_terrain_changes());
    headless
        .renderer
        .apply_skinned(client.drain_skinned_changes());
    if shot.measure > 0 {
        for (label, shadows, grass, clouds, bloom) in [
            ("base", false, false, false, false),
            ("shadows", true, false, false, false),
            ("shadows + grass", true, true, false, false),
            ("shadows + grass + clouds", true, true, true, false),
            ("shadows + grass + clouds + bloom", true, true, true, true),
        ] {
            let priced = client::Frame {
                effects: client::Effects {
                    shadows,
                    grass,
                    clouds,
                    bloom: if bloom {
                        frame.effects.bloom.max(0.5)
                    } else {
                        0.0
                    },
                    ..frame.effects
                },
                ..frame.clone()
            };
            let (median, p95) = headless.measure(&priced, shot.measure);
            println!(
                "{label}: median {median:.2} ms, p95 {p95:.2} ms (render + GPU wait, {width}x{height})"
            );
        }
    }
    let pixels = if shot.panel {
        headless.render_under(&frame, |over| {
            ui_native::Panel::headless(over.device, render::HEADLESS_FORMAT).picture(
                over.device,
                over.queue,
                over.encoder,
                over.target,
                over.size,
            );
        })
    } else {
        headless.render(&frame)
    };

    if let Some(parent) = shot.out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    write_png(&shot.out, width, height, &pixels)?;
    log::info!("{} patches -> {}", frame.patches.len(), shot.out.display());
    Ok(())
}
