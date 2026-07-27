mod cli;
mod doctor;
mod insights;
mod julia;
mod project;
mod toolchain;

use std::ffi::OsString;

pub use cli::{Command, Config};

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let config = cli::parse(args)?;

    match &config.command {
        Command::Help => {
            print!("{}", cli::help());
            Ok(())
        }
        Command::Version => {
            println!("jpm {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Init { path } => project::init(path.as_deref()),
        Command::New { path } => julia::generate(&config, path),
        Command::Doctor => {
            let project = project::discover_optional(config.project.as_deref())?;
            doctor::run(&config, project.as_deref())
        }
        Command::Use { channel } => {
            let project = project::discover_optional(config.project.as_deref())?.unwrap_or(
                std::env::current_dir()
                    .map_err(|error| format!("cannot read current directory: {error}"))?,
            );
            toolchain::select(&config, &project, channel)
        }
        Command::Fmt { paths } => {
            let project = project::discover(config.project.as_deref())?;
            julia::format(&config, &project, paths)
        }
        Command::Tree => {
            let project = project::discover(config.project.as_deref())?;
            insights::tree(&config, &project)
        }
        Command::Why { package } => {
            let project = project::discover(config.project.as_deref())?;
            insights::why(&config, &project, package)
        }
        command => {
            let project = project::discover(config.project.as_deref())?;
            julia::execute(&config, command, &project)
        }
    }
}
