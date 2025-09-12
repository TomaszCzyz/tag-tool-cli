use crate::PROJECT_DIRS;
use log::info;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{FromRow, Pool, Sqlite};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::fs;
use std::fs::OpenOptions;

type Db = Pool<Sqlite>;

#[derive(Debug)]
pub struct Storage {
    db: Db,
}

impl Storage {
    pub(crate) async fn add_tags(&self, text: &str) {
        let names = text.split(',').map(|s| s.trim());

        for name in names {
            sqlx::query("INSERT OR IGNORE INTO Tags (Text) VALUES ($1)")
                .bind(name)
                .execute(&self.db)
                .await
                .unwrap();
        }
    }
}

#[derive(FromRow, Debug)]
#[sqlx(rename_all = "PascalCase")]
pub struct Tag {
    id: u64,
    text: String,
    created_at: DateTime<Utc>,
}

impl Display for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl Storage {
    pub(crate) async fn list_tags(&self) -> Vec<Tag> {
        sqlx::query_as::<_, Tag>("SELECT Id, Text, CreatedAt FROM Tags")
            .fetch_all(&self.db)
            .await
            .unwrap()
    }
}

impl Storage {
    pub async fn initialize() -> Self {
        let db = Storage::setup_db().await;

        Self { db }
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
