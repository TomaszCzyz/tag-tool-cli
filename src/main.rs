extern crate core;

mod cli;
mod entities;
mod event_sourcing;
mod login;
mod storage;
mod tui;

use crate::cli::{Cli, Commands, TagsCommands};
use crate::event_sourcing::item_aggregate::{Item, ItemCommand, ItemEvent, ItemState};
use crate::event_sourcing::item_view::ItemView;
use crate::event_sourcing::sqlite_store::builder::SqliteStoreBuilder;
use crate::event_sourcing::sqlite_store::event_store::SqliteStore;
use crate::event_sourcing::tag_items_event_handler::TagItemsEventHandler;
use crate::event_sourcing::tag_items_view::TagItemsView;
use crate::login::LoginFlow;
use crate::storage::Storage;
use crate::tui::App;
use clap::Parser;
use color_eyre::{Report, Result};
use directories::ProjectDirs;
use esrs::AggregateState;
use esrs::manager::AggregateManager;
use esrs::store::EventStore;
use log::info;
use once_cell::sync::Lazy;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::str::FromStr;
use tokio::time::Instant;
use uuid::Uuid;

static PROJECT_DIRS: Lazy<ProjectDirs> =
    Lazy::new(|| ProjectDirs::from("com", "example", "tag-tool-cli").expect("failed to determine project directories"));

#[tokio::main]
async fn main() -> Result<()> {
    let startup_instant = Instant::now();

    // force early init of the project dirs to handle panic at startup
    let _ = PROJECT_DIRS.data_dir();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_target(false)
        .init();

    let cli = Cli::parse();

    let db = setup_db().await;

    let item_view = ItemView::new(&db).await;
    let tag_items_view = TagItemsView::new(&db).await;

    let tag_items_event_handler = TagItemsEventHandler {
        pool: db.clone(),
        view: tag_items_view.clone(),
    };

    let store: SqliteStore<Item> = SqliteStoreBuilder::new(db.clone())
        .add_event_handler(tag_items_event_handler)
        .try_build()
        .await?;

    let manager: AggregateManager<SqliteStore<Item>> = AggregateManager::new(store);

    let item_aggregate_id: Uuid = Uuid::from_str("040cadd3-1333-412d-8571-265b0188713b")?;
    // let mut aggregate_state: AggregateState<ItemState> = AggregateState::new();

    let aggregate_state = manager.load(item_aggregate_id).await?.unwrap();

    // let storage: &'static Storage = Box::leak(Box::new(Storage::initialize().await));
    // let app = App::with_storage(&storage);

    info!("startup time: {:?}", startup_instant.elapsed());

    match &cli.command {
        Commands::Tag { path, tags: tags_option } => {
            if let Some(tags) = tags_option {
                let mut input_tags = HashSet::from_iter(tags.iter().cloned());
                let item = aggregate_state.inner();
                input_tags.retain(|t| !item.tags.contains(t));

                if input_tags.is_empty() {
                    info!("The item already has tags: {:?}.", tags);
                    return Ok(());
                }

                manager
                    .handle_command(aggregate_state, ItemCommand::Tag { tags: input_tags })
                    .await??;

                Ok(())
            } else {
                // launch_tui(app)?
                Ok(())
            }
        }
        Commands::Login => {
            let flow = LoginFlow::new();
            flow.run()?;

            Ok(())
        }
        Commands::Tags { command } => Ok(match command {
            TagsCommands::List => {
                // storage.list_tags().await.iter().for_each(|s| println!("{:}", s))
            }
            TagsCommands::Add { names } => {
                // storage.add_tags(names).await;
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

async fn setup_db() -> Pool<Sqlite> {
    let mut path = PROJECT_DIRS.data_dir().to_path_buf();

    match fs::create_dir_all(path.clone()) {
        Ok(_) => {}
        Err(err) => {
            panic!("error creating directory {}", err);
        }
    };

    path.push("db.sqlite");

    let result = OpenOptions::new().create(true).write(true).open(&path);

    match result {
        Ok(_) => {}
        Err(err) => panic!("error creating database file {}", err),
    }

    let db = SqlitePoolOptions::new().connect(path.to_str().unwrap()).await.unwrap();

    sqlx::query(
        "
            PRAGMA busy_timeout = 60000;
            PRAGMA journal_mode = WAL;
            ",
    )
    .execute(&db)
    .await
    .unwrap();

    db
}
