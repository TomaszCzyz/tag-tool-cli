use crate::app_config::AppConfig;
use crate::event_sourcing::item_aggregate::Item;
use crate::event_sourcing::item_event_handler::ItemEventHandler;
use crate::event_sourcing::item_view::ItemView;
use crate::event_sourcing::setup_item_store_manager;
use crate::event_sourcing::sqlite_store::builder::SqliteStoreBuilder;
use crate::event_sourcing::sqlite_store::event_store::SqliteStore;
use crate::event_sourcing::tag_items_event_handler::TagItemsEventHandler;
use crate::event_sourcing::tag_items_view::TagItemsView;
use crate::{DbContext, PROJECT_DIRS};
use esrs::store::EventStore;
use futures::StreamExt;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::fs;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub struct DbManager;

impl DbManager {
    pub(crate) async fn setup_db(config: AppConfig) -> Pool<Sqlite> {
        let mut db_dir = config.database.path.unwrap_or(PROJECT_DIRS.data_dir().to_path_buf());

        match fs::create_dir_all(db_dir.clone()) {
            Ok(_) => {}
            Err(err) => {
                panic!("error creating directory {}", err);
            }
        };

        db_dir.push(config.database.name);

        let result = OpenOptions::new().create(true).write(true).open(&db_dir);

        match result {
            Ok(_) => {}
            Err(err) => panic!("error creating database file {}", err),
        }

        Self::connect_db(db_dir).await
    }

    async fn connect_db(db_path: PathBuf) -> Pool<Sqlite> {
        let options = SqlitePoolOptions::new();
        let db = options.connect(db_path.to_str().unwrap()).await.unwrap();

        _ = sqlx::query(
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

    pub async fn merge_dbs(input_db_paths: Vec<PathBuf>, output_db_path: PathBuf) -> color_eyre::Result<(), color_eyre::Report> {
        let output_db = Self::connect_db(output_db_path.clone()).await;

        if input_db_paths.len() < 2 {
            panic!("At least two databases are required for merging.");
        }

        if input_db_paths.len() != 2 {
            panic!("Only merging two databases is supported at this moment.");
        }

        let input_db1 = Self::merge_two_dbs(input_db_paths[0].clone(), input_db_paths[1].clone(), output_db_path).await;

        Ok(())
    }

    pub async fn merge_two_dbs(input_db1: PathBuf, input_db2: PathBuf, output_db: PathBuf) -> color_eyre::Result<(), color_eyre::Report> {
        let output_db = Self::connect_db(output_db).await;
        let input_db1 = Self::connect_db(input_db1).await;
        let input_db2 = Self::connect_db(input_db2).await;

        let output_item_view = ItemView::new(&output_db).await;
        let output_tag_items_view = TagItemsView::new(&output_db).await;

        let item_event_handler = ItemEventHandler {
            pool: output_db.clone(),
            view: output_item_view.clone(),
        };

        let tag_items_event_handler = TagItemsEventHandler {
            pool: output_db.clone(),
            view: output_tag_items_view.clone(),
        };

        let output_store: SqliteStore<Item> = SqliteStoreBuilder::new(output_db.clone())
            .add_event_handler(item_event_handler)
            .add_event_handler(tag_items_event_handler)
            .try_build()
            .await?;

        // output_store.persist().await?;

        let input_store1: SqliteStore<Item, _> = SqliteStoreBuilder::new(input_db1.clone()).try_build().await?;
        let input_store2: SqliteStore<Item, _> = SqliteStoreBuilder::new(input_db2.clone()).try_build().await?;

        let mut events_stream1 = input_store1.stream_events(&input_db1);
        let mut events_stream2 = input_store2.stream_events(&input_db2);

        // TODO: handle empty streams
        let mut curret_event1 = events_stream1.next().await.unwrap()?;
        let mut curret_event2 = events_stream2.next().await.unwrap()?;
        loop {
            if curret_event1.occurred_on < curret_event2.occurred_on {
                // let state;
                // output_store.persist(state, ).await?;
                match events_stream1.next().await {
                    Some(event) => {
                        curret_event1 = event?;
                    }
                    None => break,
                }
            } else {
                todo!()
                // output_store.persist_event(&output_db, &curret_event2).await?;
                // match events_stream2.next().await {
                //     Some(event) => {
                //         curret_event2 = event?;
                //     }
                //     None => break,
                // }
            }
        }

        Ok(())
    }
}
