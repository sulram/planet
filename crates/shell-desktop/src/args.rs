//! Command line parsing. Small enough that a dependency would be heavier.

use std::path::PathBuf;

use worldgen::{Field, Recipe, Source, parse_seed};

pub enum Invocation {
    Window {
        recipe: Recipe,
        field: Option<Field>,
        avatar: Option<String>,
        /// Where to stand in sector 0, each `0..=1`. `None` is the spawn.
        at: Option<String>,
    },
    Shot(Shot),
}

pub struct Shot {
    pub recipe: Recipe,
    /// The ground the recipe names, already read. `None` for a seed world.
    pub field: Option<Field>,
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
    pub at: Option<String>,
    /// Fly to this many metres under the moon instead.
    pub moon_gap_m: Option<f64>,
    /// Fixed-scene render timing, with effects toggled for comparison.
    pub measure: usize,
    /// Paint the native settings panel, open, over the picture.
    pub panel: bool,
    /// Effects as the seam takes them: `{"tone_map":"agx","bloom":1}`.
    pub effects: Option<String>,
    /// Paint a vertical cut through the ground instead of rendering it, this
    /// many metres wide. Zero renders the world as usual.
    pub slice_m: f64,
    /// What a hand does before the shot, in the order given.
    pub steps: Vec<Step>,
}

/// One thing a hand does before a shot is taken.
pub enum Step {
    /// A command as the seam takes it: `{"type":"set_tool","tool":"create"}`.
    Command(String),
    /// Press the pointer's button at one point of the view and let go at
    /// another, fractions from the top left: a stroke of the tool in hand.
    Drag([f32; 4]),
}

/// The seed every preview starts from unless told otherwise.
const DEFAULT_SEED: u64 = 1;

pub fn parse(args: impl Iterator<Item = String>) -> Result<Invocation, String> {
    let mut args = args.peekable();
    let is_shot = args.next_if(|arg| arg == "shot").is_some();

    let mut recipe = Recipe::new(DEFAULT_SEED);
    let mut field = None;
    let mut shot = Shot {
        recipe: recipe.clone(),
        field: None,
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
        effects: None,
        slice_m: 0.0,
        steps: Vec::new(),
    };

    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--seed" => recipe.seed = parse_seed(&value()?).map_err(|e| e.to_string())?,
            // The body's size, `4..=16`. A small world fits whole inside the
            // near field, which is the only place there is ground yet.
            "--bits" => recipe.sector_bits = number(&value()?, "--bits")?,
            // The file names the ground, and the recipe follows it: a field
            // world is only ever the world that field was baked for.
            "--field" => {
                let path = value()?;
                let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
                let read = Field::parse(bytes).map_err(|e| format!("{path}: {e}"))?;
                recipe.params.source = Source::Field(read.id());
                field = Some(read);
            }
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
            "--slice" if is_shot => shot.slice_m = number(&value()?, "--slice")?,
            "--effects" if is_shot => shot.effects = Some(value()?),
            "--command" if is_shot => shot.steps.push(Step::Command(value()?)),
            "--drag" if is_shot => {
                let text = value()?;
                let parts: Vec<f32> = text
                    .split(',')
                    .map(|part| number(part, "--drag"))
                    .collect::<Result<_, _>>()?;
                let points: [f32; 4] = parts
                    .try_into()
                    .map_err(|_| "--drag wants X0,Y0,X1,Y1".to_owned())?;
                shot.steps.push(Step::Drag(points));
            }
            "--clock" if is_shot => shot.clock_s = number(&value()?, "--clock")?,
            "--altitude" if is_shot => shot.altitude_m = number(&value()?, "--altitude")?,
            "--pitch" if is_shot => shot.pitch_deg = number(&value()?, "--pitch")?,
            // The one place flag both invocations take: a picture and a walk
            // are worth nothing to each other if they cannot be aimed alike.
            // A place code, as the HUD and the address bar show it. Two
            // fractions of sector 0 used to go here, which could not name the
            // other five and which nobody could read back off a screenshot.
            "--at" => shot.at = Some(value()?),
            "--moon" if is_shot => shot.moon_gap_m = Some(number(&value()?, "--moon")?),
            "--walk" if is_shot => shot.walk_s = number(&value()?, "--walk")?,
            "--boom" if is_shot => shot.boom_m = number(&value()?, "--boom")?,
            other => return Err(format!("unknown argument {other}")),
        }
    }

    if !is_shot {
        return Ok(Invocation::Window {
            recipe,
            field,
            avatar: shot.avatar,
            at: shot.at,
        });
    }
    if shot.out.as_os_str().is_empty() {
        return Err("shot needs --out FILE".into());
    }
    shot.recipe = recipe;
    shot.field = field;
    Ok(Invocation::Shot(shot))
}

fn number<T: std::str::FromStr>(text: &str, flag: &str) -> Result<T, String> {
    text.parse()
        .map_err(|_| format!("{flag}: cannot read {text:?} as a number"))
}
