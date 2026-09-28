//! The desktop shell: the offline explorer in a window, or a headless shot.
//!
//! ```text
//! planet [--seed HEX] [--bits N] [--avatar NAME] [--at U,V]
//! planet shot --out FILE [--seed HEX] [--bits N] [--avatar NAME] [--size WxH] [--clock S]
//!             [--altitude M] [--pitch DEG] [--boom M] [--walk S] [--at U,V] [--moon M]
//!             [--command JSON]... [--drag X0,Y0,X1,Y1]...
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
            at,
        }) => window::run(recipe, field, avatar, at),
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
