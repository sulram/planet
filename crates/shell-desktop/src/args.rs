//! Command line parsing. Small enough that a dependency would be heavier.

use std::path::PathBuf;

use worldgen::{Recipe, parse_seed};

pub enum Invocation {
    Window { recipe: Recipe },
    Shot(Shot),
}

pub struct Shot {
    pub recipe: Recipe,
    pub out: PathBuf,
    pub size: [u32; 2],
    /// Seconds on the world clock: fixes the sun.
    pub clock_s: f64,
    pub altitude_m: f64,
    pub pitch_deg: f64,
    pub boom_m: f64,
}

/// The seed every preview starts from unless told otherwise.
const DEFAULT_SEED: u64 = 1;

pub fn parse(args: impl Iterator<Item = String>) -> Result<Invocation, String> {
    let mut args = args.peekable();
    let is_shot = args.next_if(|arg| arg == "shot").is_some();

    let mut recipe = Recipe::new(DEFAULT_SEED);
    let mut shot = Shot {
        recipe: recipe.clone(),
        out: PathBuf::new(),
        size: [1280, 720],
        clock_s: 0.0,
        altitude_m: 0.0,
        pitch_deg: -14.0,
        boom_m: 6.0,
    };

    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--seed" => recipe.seed = parse_seed(&value()?).map_err(|e| e.to_string())?,
            "--out" if is_shot => shot.out = PathBuf::from(value()?),
            "--size" if is_shot => {
                let text = value()?;
                let (w, h) = text.split_once('x').ok_or("--size wants WxH")?;
                shot.size = [number(w, "--size")?, number(h, "--size")?];
            }
            "--clock" if is_shot => shot.clock_s = number(&value()?, "--clock")?,
            "--altitude" if is_shot => shot.altitude_m = number(&value()?, "--altitude")?,
            "--pitch" if is_shot => shot.pitch_deg = number(&value()?, "--pitch")?,
            "--boom" if is_shot => shot.boom_m = number(&value()?, "--boom")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }

    if !is_shot {
        return Ok(Invocation::Window { recipe });
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
