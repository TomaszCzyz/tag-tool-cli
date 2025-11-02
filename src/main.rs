#![allow(dead_code)]
extern crate core;

mod cli;
mod event_sourcing;
mod login;
mod tui;
mod tui_view;

use crate::cli::{Cli, Commands, ItemsCommands, TagsCommands};
use crate::event_sourcing::item_aggregate::{Item, ItemCommand, ItemEvent};
use crate::event_sourcing::item_event_handler::ItemEventHandler;
use crate::event_sourcing::item_view::ItemView;
use crate::event_sourcing::sqlite_store::builder::SqliteStoreBuilder;
use crate::event_sourcing::sqlite_store::event_store::SqliteStore;
use crate::event_sourcing::tag_items_event_handler::TagItemsEventHandler;
use crate::event_sourcing::tag_items_view::TagItemsView;
use crate::login::LoginFlow;
use crate::tui::App;
use blake3::Hash;
use clap::Parser;
use color_eyre::{Report, Result};
use crossterm::event::{Event, KeyCode, KeyEvent, read};
use directories::{ProjectDirs, UserDirs};
use esrs::AggregateState;
use esrs::manager::AggregateManager;
use log::info;
use once_cell::sync::Lazy;
use same_file::is_same_file;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;
use std::path::PathBuf;
use tokio::time::Instant;
use tracing::error;
use uuid::Uuid;

static PROJECT_DIRS: Lazy<ProjectDirs> =
    Lazy::new(|| ProjectDirs::from("com", "example", "tag-tool-cli").expect("failed to determine project directories"));
static USER_DIRS: Lazy<UserDirs> = Lazy::new(|| UserDirs::new().expect("failed to determine user directories"));

#[tokio::main]
async fn main() -> Result<()> {
    let startup_instant = Instant::now();

    // force early init of the project dirs to handle panic at startup
    let _ = PROJECT_DIRS.data_dir();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .format_timestamp_millis()
        .format_target(false)
        .init();

    let db = setup_db().await;

    let item_view = ItemView::new(&db).await;
    let tag_items_view = TagItemsView::new(&db).await;
    let manager = setup_item_store_manager(&db, &item_view, &tag_items_view).await?;

    info!("startup time: {:?}", startup_instant.elapsed());

    let cli = Cli::parse();
    match &cli.command {
        Commands::Tag {
            path,
            tags: tags_option,
            move_to_common_storage,
        } => {
            if !path.exists() {
                info!("File does not exist: {:?}", path);
                return Ok(());
            }

            if let Some(tags) = tags_option {
                let hash = hash_file(path);
                let aggregate_id = if let Some(item) = item_view.find_by_hash(&hash, &db).await? {
                    info!("Found item: {:}", item);
                    // TODO: validate case, when paths are the same, but contents are different
                    if let Ok(is_same) = is_same_file(&item.path, path) {
                        if !is_same {
                            panic!("File with the same content is already tracked")
                        }
                    };
                    item.id
                } else {
                    info!("Creating new item: {:?}", path);
                    create_item(&manager, path, hash).await?
                };
                let aggregate_state = manager.load(aggregate_id).await?.unwrap();

                let mut input_tags = HashSet::from_iter(tags.iter().cloned());
                input_tags.retain(|t| !aggregate_state.inner().tags.contains(t));

                if input_tags.is_empty() {
                    info!("The item already has tags: {:?}.", tags);

                    if *move_to_common_storage {
                        move_item_to_common_storage(aggregate_id, manager).await?;
                    }

                    return Ok(());
                }

                manager
                    .handle_command(aggregate_state, ItemCommand::Tag { tags: input_tags })
                    .await??;

                if *move_to_common_storage {
                    move_item_to_common_storage(aggregate_id, manager).await?;
                }

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
                for tag in tag_items_view.get_all_tags(&db).await? {
                    println!("{:}", tag);
                }
            }
        }),
        Commands::Search => {
            let app = App::from(db, Box::new(tag_items_view));
            launch_tui(app).await??;

            Ok(())
        }
        Commands::Items { .. } => Ok(()),
    }
}

async fn move_item_to_common_storage(aggregate_id: Uuid, manager: AggregateManager<SqliteStore<Item, ItemEvent>>) -> Result<(), Report> {
    info!("Moving item to common storage: {:?}", aggregate_id);
    let aggregate_state = manager.load(aggregate_id).await?.unwrap();
    if let Some(doc_path) = USER_DIRS.document_dir() {
        let item_path = PathBuf::from(&aggregate_state.inner().path).canonicalize()?;
        let original_file_name = item_path
            .file_name()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "source path has no file name"))?;

        let mut dest_path_buf = doc_path.to_path_buf();
        dest_path_buf.push("tag-tool");
        dest_path_buf.push("common-storage");

        fs::create_dir_all(&dest_path_buf).expect("Failed to create directory");

        dest_path_buf.push(original_file_name);

        if dest_path_buf.exists() {
            println!("File already exists in common storage: {:?}", dest_path_buf);
            println!("Do you want to override it? [y/N]");

            match read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('y'), ..
                }) => (),
                Event::Key(KeyEvent {
                    code: KeyCode::Char('Y'), ..
                }) => (),
                _ => return Ok(()),
            }
        }

        match fs::rename(item_path, &dest_path_buf) {
            Ok(_) => {
                manager
                    .handle_command(
                        aggregate_state,
                        ItemCommand::Move {
                            new_path: dest_path_buf.to_string_lossy().to_string(),
                        },
                    )
                    .await??;
            }
            Err(e) => {
                error!("Failed to move file to common storage: {:?}, error: {}", dest_path_buf, e);
            }
        }
    }
    Ok(())
}

async fn setup_item_store_manager(
    db: &Pool<Sqlite>,
    item_view: &ItemView,
    tag_items_view: &TagItemsView,
) -> Result<AggregateManager<SqliteStore<Item, ItemEvent>>, Report> {
    let item_event_handler = ItemEventHandler {
        pool: db.clone(),
        view: item_view.clone(),
    };

    let tag_items_event_handler = TagItemsEventHandler {
        pool: db.clone(),
        view: tag_items_view.clone(),
    };

    let store: SqliteStore<Item> = SqliteStoreBuilder::new(db.clone())
        .add_event_handler(item_event_handler)
        .add_event_handler(tag_items_event_handler)
        .try_build()
        .await?;

    Ok(AggregateManager::new(store))
}

async fn create_item(manager: &AggregateManager<SqliteStore<Item, ItemEvent>>, path: &PathBuf, hash: Hash) -> Result<Uuid, Report> {
    let new_aggregate_id = Uuid::new_v4();
    let aggregate_state = AggregateState::with_id(new_aggregate_id);
    manager
        .handle_command(
            aggregate_state,
            ItemCommand::CreateItem {
                hash,
                path: path.to_string_lossy().to_string(),
            },
        )
        .await??;

    Ok(new_aggregate_id)
}

fn hash_file(path: &PathBuf) -> Hash {
    let mut hasher = blake3::Hasher::new();
    match hasher.update_mmap(path) {
        Ok(_) => {}
        Err(e) => panic!("Failed to hash file: {}", e),
    }
    let hash = hasher.finalize();
    hash
}

async fn launch_tui(app: App) -> Result<Result<()>, Report> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
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
