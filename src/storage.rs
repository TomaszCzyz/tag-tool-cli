use crate::PROJECT_DIRS;
use crate::entities::{File, Tag};
use blake3::Hash;
use log::info;
use same_file::is_same_file;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{Pool, Sqlite, query_as};
use std::fs;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

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
        let file = self.get_or_add_file(path).await;

        for tag in tags {
            if file.tags.contains(&tag) {
                info!("file {} already has tag {}", file, tag);
                continue;
            }
            sqlx::query("INSERT INTO TagsFiles (FileId, TagId) VALUES ($1, $2)")
                .bind(&file.id)
                .bind(&tag.id)
                .execute(&self.db)
                .await
                .unwrap();
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
                    info!("adding a new tag {}", tag_text);

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

    async fn get_or_add_file(&self, path_buf: &PathBuf) -> File {
        let path = Path::new(path_buf);
        let mut hasher = blake3::Hasher::new();
        match hasher.update_mmap(path) {
            Ok(_) => {}
            Err(e) => panic!("Failed to hash file: {}", e),
        }
        let hash = hasher.finalize();

        let result = self.find_file_by_hash_with_tags(&hash).await.unwrap();

        match result {
            Some(db_file) => match is_same_file(&db_file.path, path_buf) {
                Ok(is_same) => {
                    if is_same {
                        db_file
                    } else {
                        panic!("the same file, but with different content is already tagged")
                    }
                }
                Err(e) => panic!("{}", e),
            },
            None => {
                info!("adding a new file {}", path_buf.to_string_lossy());

                let path_normalized = path_buf.canonicalize().unwrap();
                let file = sqlx::query_as::<_, File>(
                    "INSERT INTO Files (Path, Hash) VALUES ($1, $2) RETURNING Id, Path, Hash, CreatedAt",
                )
                    .bind(&path_normalized.to_string_lossy())
                    .bind(&hash.as_bytes()[..])
                    .fetch_one(&self.db)
                    .await
                    .unwrap();

                file
            }
        }
    }

    pub(crate) async fn find_file_by_hash_with_tags(
        &self,
        hash: &Hash,
    ) -> color_eyre::Result<Option<File>> {
        use sqlx::Row;

        let rows = sqlx::query(
            r#"
            SELECT
                f.Id          AS FileId,
                f.Path        AS FilePath,
                f.Hash        AS FileHash,
                f.CreatedAt   AS FileCreatedAt,
                t.Id          AS TagId,
                t.Text        AS TagText,
                t.CreatedAt   AS TagCreatedAt
            FROM Files f
            LEFT JOIN TagsFiles tf ON tf.FileId = f.Id
            LEFT JOIN Tags t       ON t.Id = tf.TagId
            WHERE f.Hash = ?
            "#,
        )
            .bind(&hash.as_bytes()[..])
            .fetch_all(&self.db)
            .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let first = &rows[0];
        let mut file = File {
            id: first.try_get::<i64, _>("FileId")?,
            path: first.try_get::<String, _>("FilePath")?,
            hash: first.try_get::<Vec<u8>, _>("FileHash")?.into_boxed_slice(),
            created_at: first.try_get::<DateTime<Utc>, _>("FileCreatedAt")?,
            tags: Vec::new(),
        };

        for row in rows {
            let tag_id: Option<i64> = row.try_get("TagId").ok();
            if let Some(tag_id) = tag_id {
                let tag = Tag {
                    id: tag_id,
                    text: row.try_get::<String, _>("TagText")?,
                    created_at: row.try_get::<DateTime<Utc>, _>("TagCreatedAt")?,
                };
                file.tags.push(tag);
            }
        }

        Ok(Some(file))
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
