use crate::PROJECT_DIRS;
use log::info;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::borrow::Cow;
use std::collections::HashSet;
use std::fs;
use std::fs::OpenOptions;

type Db = Pool<Sqlite>;

#[derive(Debug)]
pub struct Storage {
    db: Db,

    /// All user defined tags.
    tags: HashSet<Cow<'static, str>>,
}

impl Storage {
    pub(crate) fn add(&self, p0: Cow<'static, str>) {
        todo!()
    }
}

impl Storage {
    pub(crate) fn list(&self) -> Vec<Cow<'static, str>> {
        todo!()
    }
}

impl Storage {
    pub async fn initialize() -> Self {
        let db = Storage::setup_db().await;

        Self {
            db,
            tags: HashSet::new(),
        }
    }

    async fn setup_db() -> Db {
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

        let db = SqlitePoolOptions::new()
            .connect(path.to_str().unwrap())
            .await
            .unwrap();

        info!("Executing migrations...");
        sqlx::migrate!("src/migrations").run(&db).await.unwrap();

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
}
