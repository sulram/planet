//! The desktop shell: the offline explorer in a window, or a headless shot.
//!
//! ```text
//! planet [--seed HEX] [--avatar NAME]
//! planet shot --out FILE [--seed HEX] [--avatar NAME] [--size WxH] [--clock S]
//!             [--altitude M] [--pitch DEG] [--boom M] [--walk S] [--at U,V] [--moon M]
//! ```

mod args;
mod assets;
mod shot;
mod slice;
mod window;

use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,planet=info"))
        .init();
    let result = match args::parse(std::env::args().skip(1)) {
        Ok(args::Invocation::Window {
            recipe,
            field,
            avatar,
        }) => window::run(recipe, field, avatar),
        Ok(args::Invocation::Shot(shot)) => shot::run(shot),
        Err(message) => Err(message),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("planet: {message}");
            ExitCode::FAILURE
        }
    }
}
