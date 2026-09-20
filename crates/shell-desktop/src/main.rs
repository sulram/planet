//! The desktop shell: the offline explorer in a window, or a headless shot.
//!
//! ```text
//! planet [--seed HEX]
//! planet shot --out FILE [--seed HEX] [--size WxH] [--clock S]
//!             [--altitude M] [--pitch DEG] [--boom M]
//! ```

mod args;
mod shot;
mod window;

use std::process::ExitCode;

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn,planet=info"))
        .init();
    let result = match args::parse(std::env::args().skip(1)) {
        Ok(args::Invocation::Window { recipe }) => window::run(recipe),
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
