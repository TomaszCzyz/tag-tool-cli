use crate::PROJECT_DIRS;
use crate::entities::Tag;
use log::info;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite, query_as};
use std::fs;
use std::fs::OpenOptions;
use std::path::PathBuf;

type Db = Pool<Sqlite>;

#[derive(Debug)]
pub struct Storage {
    db: Db,
}

impl Storage {
    pub(crate) async fn tag_file(&self, path: &PathBuf, tag_names: &Vec<String>) {
        if path.is_dir() {
            panic!("path is a directory");
        }

        if tag_names.is_empty() {
            return;
        }

        let tags = self.get_or_add_tags(tag_names).await;

        for tag in tags {
            info!("{tag:?}")
        }
    }

    async fn get_or_add_tags(&self, tag_names: &Vec<String>) -> Vec<Tag> {
        let mut tags = Vec::new();

        for tag_text in tag_names {
            let result = query_as::<_, Tag>("SELECT Id, Text, CreatedAt FROM Tags WHERE Text = $1")
                .bind(tag_text)
                .fetch_optional(&self.db)
                .await
                .unwrap();

            match result {
                None => {
                    info!("adding a new tag {tag_text}");

                    let tag = sqlx::query_as::<_, Tag>(
                        "INSERT INTO Tags (Text) VALUES ($1) RETURNING Id, Text, CreatedAt",
                    )
                        .bind(tag_text)
                        .fetch_one(&self.db)
                        .await
                        .unwrap();

                    tags.push(tag);
                }
                Some(tag) => {
                    tags.push(tag);
                }
            }
        }

        tags
    }

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

    pub(crate) async fn list_tags(&self) -> Vec<Tag> {
        query_as::<_, Tag>("SELECT Id, Text, CreatedAt FROM Tags")
            .fetch_all(&self.db)
            .await
            .unwrap()
    }

    pub(crate) async fn initialize() -> Self {
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
