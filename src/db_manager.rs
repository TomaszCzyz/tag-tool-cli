use crate::PROJECT_DIRS;
use crate::app_config::AppConfig;
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

        let options = SqlitePoolOptions::new();
        let db = options.connect(db_dir.to_str().unwrap()).await.unwrap();

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

    pub async fn merge_dbs(input_dbs: Vec<PathBuf>, output_db: PathBuf) -> color_eyre::Result<(), color_eyre::Report> {
        // Placeholder for the merge logic
        Ok(())
    }
}
