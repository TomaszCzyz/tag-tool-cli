mod cli;
mod tags_storage;
mod tui;

use crate::cli::{Cli, Commands, TagsCommands};
use crate::tags_storage::Storage;
use crate::tui::App;
use clap::Parser;
use color_eyre::{Report, Result};
use directories::ProjectDirs;
use log::info;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use tokio::time::Instant;

static PROJECT_DIRS: Lazy<ProjectDirs> = Lazy::new(|| {
    ProjectDirs::from("com", "example", "tag-tool-cli")
        .expect("failed to determine project directories")
});

#[tokio::main]
async fn main() -> Result<()> {
    let startup_instant = Instant::now();
    // force early init of the project dirs to handle panic
    let _ = PROJECT_DIRS.data_dir();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let cli = Cli::parse();

    let storage: &'static Storage = Box::leak(Box::new(Storage::initialize().await));
    let app = App::with_storage(&storage);

    info!("startup time: {:?}", startup_instant.elapsed());

    match &cli.command {
        Commands::Tag { path, tags } => {
            if let Some(tags) = tags {
                println!("{:?}", tags);
                Ok(())
            } else {
                launch_tui(app)?
            }
        }
        Commands::Tags { command } => Ok(match command {
            TagsCommands::List => storage.list().iter().for_each(|s| println!("{}", s)),
            TagsCommands::Add { name } => {
                storage.add(Cow::Owned(name.to_string()));
            }
        }),
        Commands::Items { command } => Ok(match command {
            _ => {}
        }),
    }
}

fn launch_tui(app: App) -> Result<Result<()>, Report> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app.run(terminal);
    ratatui::restore();
    Ok(result)
}
