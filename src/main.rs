#![allow(dead_code)]
extern crate core;

mod app_config;
mod cli;
mod db_manager;
mod event_sourcing;
mod items_tagger;
mod login;
mod tag;
mod tag_query;
mod tuis;
mod utils;

use crate::app_config::AppConfig;
use crate::cli::{Cli, Commands, ItemsCommands, TagsCommands};
use crate::db_manager::DbManager;
use crate::event_sourcing::item_aggregate::{Item, ItemEvent};
use crate::event_sourcing::item_event_handler::ItemEventHandler;
use crate::event_sourcing::item_view::ItemView;
use crate::event_sourcing::sqlite_store::builder::SqliteStoreBuilder;
use crate::event_sourcing::sqlite_store::event_store::SqliteStore;
use crate::event_sourcing::tag_items_event_handler::TagItemsEventHandler;
use crate::event_sourcing::tag_items_view::TagItemsView;
use crate::items_tagger::ItemsTagger;
use crate::login::LoginFlow;
use crate::tuis::search::app::TagSearchTui;
use crate::tuis::tag::app::TagTui;
use clap::Parser;
use color_eyre::{Report, Result};
use directories::{ProjectDirs, UserDirs};
use esrs::manager::AggregateManager;
use log::info;
use once_cell::sync::Lazy;
use ratatui::{TerminalOptions, Viewport};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::fs;
use std::fs::OpenOptions;
use std::path::Path;
use tokio::time::Instant;

static PROJECT_DIRS: Lazy<ProjectDirs> =
    Lazy::new(|| ProjectDirs::from("com", "example", "tag-tool-cli").expect("failed to determine project directories"));
static USER_DIRS: Lazy<UserDirs> = Lazy::new(|| UserDirs::new().expect("failed to determine user directories"));

#[derive(Clone)]
struct DbContext {
    db: Pool<Sqlite>,
    item_view: ItemView,
    tag_items_view: TagItemsView,
}

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

    let app_config = AppConfig::load_from_file("./tagtool_config.toml")?;

    println!("app config: {:?}", app_config);

    let db = DbManager::setup_db(app_config).await;

    let item_view = ItemView::new(&db).await;
    let tag_items_view = TagItemsView::new(&db).await;

    let db_ctx = DbContext {
        db: db.clone(),
        item_view: item_view.clone(),
        tag_items_view: tag_items_view.clone(),
    };

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

            let path = path.canonicalize()?;

            if let Some(tags) = tags_option {
                let tagger = ItemsTagger::initialize(db_ctx.clone()).await;
                tagger.tag_item(path.as_path(), tags, *move_to_common_storage).await
            } else {
                let app = TagTui::from(db_ctx, path.to_path_buf()).await;
                launch_tag_tui(app).await?
            }
        }
        Commands::Untag { path, tags: tags_option } => {
            let path = path.canonicalize()?;
            if let Some(tags) = tags_option {
                let tagger = ItemsTagger::initialize(db_ctx.clone()).await;
                tagger.untag_item(path.as_path(), tags).await?
            }
            Ok(())
        }
        Commands::Login => {
            let flow = LoginFlow::new();
            flow.run()
        }
        Commands::Tags { command } => Ok(match command {
            TagsCommands::List => {
                for tag in tag_items_view.get_all_tags(&db).await? {
                    println!("{:}", tag);
                }
            }
        }),
        Commands::Search => {
            let app = TagSearchTui::from(db_ctx);
            launch_search_tui(app).await?
        }
        Commands::Items { command } => match command {
            ItemsCommands::List { tag_query } => {
                let items = tag_items_view.list(tag_query.clone(), &db).await?;
                for item in items {
                    let path = Path::new(&item.path);
                    println!("{:?} {}", path.file_name().unwrap_or_default(), item.tags);
                }
                Ok(())
            }
            ItemsCommands::Merge { input_dbs, output_db } => {
                let output_db_path = output_db.as_ref().expect("unsupported: missing output db");
                
                
                Ok(())
            }
        },
    }
}

async fn setup_item_store_manager(db_ctx: DbContext) -> Result<AggregateManager<SqliteStore<Item, ItemEvent>>, Report> {
    let item_event_handler = ItemEventHandler {
        pool: db_ctx.db.clone(),
        view: db_ctx.item_view.clone(),
    };

    let tag_items_event_handler = TagItemsEventHandler {
        pool: db_ctx.db.clone(),
        view: db_ctx.tag_items_view.clone(),
    };

    let store: SqliteStore<Item> = SqliteStoreBuilder::new(db_ctx.db.clone())
        .add_event_handler(item_event_handler)
        .add_event_handler(tag_items_event_handler)
        .try_build()
        .await?;

    Ok(AggregateManager::new(store))
}

async fn launch_search_tui(app: TagSearchTui) -> Result<Result<()>, Report> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    Ok(result)
}

async fn launch_tag_tui(app: TagTui) -> Result<Result<()>, Report> {
    let terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(15),
    });
    let _t = terminal.size()?;

    color_eyre::install()?;
    let result = app.run(terminal).await;

    ratatui::restore();
    Ok(result)
}
