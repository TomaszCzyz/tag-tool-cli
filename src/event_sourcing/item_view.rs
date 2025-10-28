use blake3::Hash;
use sqlx::{Executor, Pool, Sqlite, query_as};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(sqlx::FromRow, Debug)]
pub struct ItemViewRow {
    pub id: Uuid,
    pub path: String,
    pub hash: Box<[u8]>,
}

#[derive(Clone)]
pub struct ItemView;

#[allow(dead_code)]
impl ItemView {
    const TABLE_NAME: &'static str = "item_view";

    pub async fn new(pool: &Pool<Sqlite>) -> Self {
        let query: String = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}
            (
                id        TEXT NOT NULL,
                path      TEXT NOT NULL UNIQUE,
                hash      BLOB NULL,

                PRIMARY KEY (id)
            )
            "#,
            Self::TABLE_NAME
        );

        let _ = sqlx::query(query.as_str()).execute(pool).await.unwrap();

        Self
    }

    pub async fn by_id(&self, id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<Option<ItemViewRow>, sqlx::Error> {
        let query: String = format!("SELECT * FROM {} WHERE id = $1", Self::TABLE_NAME);

        todo!()

        // sqlx::query_as::<_, ItemViewRow>(query.as_str())
        //     .bind(id)
        //     .fetch_optional(executor)
        //     .await
    }

    pub async fn upsert(
        &self,
        id: Uuid,
        path_buf: PathBuf,
        hash: Box<[u8]>,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        let path_normalized = path_buf.canonicalize()?;

        let query = format!(
            r#"
            INSERT INTO {0}
            (
                id,
                path,
                hash
            )
            VALUES
                (?, ?, ?)
            ON CONFLICT DO UPDATE SET
                path = excluded.path,
                hash = excluded.hash
            "#,
            Self::TABLE_NAME
        );

        sqlx::query(query.as_str())
            .bind(id)
            .bind(&path_normalized.to_string_lossy())
            .bind(&hash[..])
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn find_by_hash(&self, hash: &Hash, executor: impl Executor<'_, Database = Sqlite>) -> Result<Option<ItemViewRow>, sqlx::Error> {
        let rows = query_as::<_, ItemViewRow>(
            r#"
            SELECT
                id,
                path,
                hash
            FROM item_view
            WHERE hash = ?
            "#,
        )
        .bind(&hash.as_bytes()[..])
        .fetch_all(executor)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let r = rows.into_iter().next().unwrap();
        Ok(Some(r))
    }

    pub async fn delete(&self, id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<(), sqlx::Error> {
        let query = format!("DELETE FROM {0} WHERE id = $1;", Self::TABLE_NAME);

        sqlx::query(query.as_str()).bind(id).fetch_optional(executor).await.map(|_| ())
    }
}
