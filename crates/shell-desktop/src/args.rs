//! Command line parsing. Small enough that a dependency would be heavier.

use std::path::PathBuf;

use worldgen::{Recipe, parse_seed};

pub enum Invocation {
    Window {
        recipe: Recipe,
        avatar: Option<String>,
    },
    Shot(Shot),
}

pub struct Shot {
    pub recipe: Recipe,
    /// File stem under `assets/avatars`.
    pub avatar: Option<String>,
    pub out: PathBuf,
    pub size: [u32; 2],
    /// Seconds on the world clock: fixes the sun.
    pub clock_s: f64,
    pub altitude_m: f64,
    pub pitch_deg: f64,
    pub boom_m: f64,
    /// Seconds of walking forward before the shot, to catch a gait mid stride.
    pub walk_s: f64,
    /// Where to stand in sector 0, each `0..=1`. Default: the world's spawn.
    pub at: Option<[f64; 2]>,
    /// Fly to this many metres under the moon instead.
    pub moon_gap_m: Option<f64>,
    /// Fixed-scene render timing, with effects toggled for comparison.
    pub measure: usize,
    /// Paint the native settings panel, open, over the picture.
    pub panel: bool,
}

/// The seed every preview starts from unless told otherwise.
const DEFAULT_SEED: u64 = 1;

pub fn parse(args: impl Iterator<Item = String>) -> Result<Invocation, String> {
    let mut args = args.peekable();
    let is_shot = args.next_if(|arg| arg == "shot").is_some();

    let mut recipe = Recipe::new(DEFAULT_SEED);
    let mut shot = Shot {
        recipe: recipe.clone(),
        avatar: None,
        out: PathBuf::new(),
        size: [1280, 720],
        clock_s: 0.0,
        altitude_m: 0.0,
        pitch_deg: -14.0,
        boom_m: 6.0,
        walk_s: 0.0,
        at: None,
        moon_gap_m: None,
        measure: 0,
        panel: false,
    };

    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--seed" => recipe.seed = parse_seed(&value()?).map_err(|e| e.to_string())?,
            "--avatar" => shot.avatar = Some(value()?),
            "--out" if is_shot => shot.out = PathBuf::from(value()?),
            "--size" if is_shot => {
                let text = value()?;
                let (w, h) = text.split_once('x').ok_or("--size wants WxH")?;
                shot.size = [number(w, "--size")?, number(h, "--size")?];
            }
            "--measure" if is_shot => {
                shot.measure = number::<usize>(&value()?, "--measure")?.min(1000)
            }
            "--panel" if is_shot => shot.panel = true,
            "--clock" if is_shot => shot.clock_s = number(&value()?, "--clock")?,
            "--altitude" if is_shot => shot.altitude_m = number(&value()?, "--altitude")?,
            "--pitch" if is_shot => shot.pitch_deg = number(&value()?, "--pitch")?,
            "--at" if is_shot => {
                let text = value()?;
                let (u, v) = text.split_once(',').ok_or("--at wants U,V")?;
                shot.at = Some([number(u, "--at")?, number(v, "--at")?]);
            }
            "--moon" if is_shot => shot.moon_gap_m = Some(number(&value()?, "--moon")?),
            "--walk" if is_shot => shot.walk_s = number(&value()?, "--walk")?,
            "--boom" if is_shot => shot.boom_m = number(&value()?, "--boom")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }

    if !is_shot {
        return Ok(Invocation::Window {
            recipe,
            avatar: shot.avatar,
        });
    }
    if shot.out.as_os_str().is_empty() {
        return Err("shot needs --out FILE".into());
    }
    shot.recipe = recipe;
    Ok(Invocation::Shot(shot))
}

fn number<T: std::str::FromStr>(text: &str, flag: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("{flag}: cannot read {text:?} as a number"))
}
