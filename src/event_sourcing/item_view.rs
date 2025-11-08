use blake3::Hash;
use log::warn;
use sqlx::{Executor, Pool, Sqlite, query_as};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use uuid::Uuid;

#[allow(unused)]
#[derive(sqlx::FromRow, Debug)]
pub struct ItemViewRow {
    pub id: Uuid,
    pub path: String,
    pub hash: Box<[u8]>,
    pub tags: String,
}

impl Display for ItemViewRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.id, self.path, self.tags)
    }
}

#[derive(Clone)]
pub struct ItemView;

#[allow(dead_code)]
impl ItemView {
    const TABLE_NAME: &'static str = "view_items";

    pub async fn new(pool: &Pool<Sqlite>) -> Self {
        let query: String = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}
            (
                id        TEXT NOT NULL,
                path      TEXT NOT NULL UNIQUE,
                hash      BLOB NULL,
                tags      TEXT NULL,

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

        sqlx::query_as::<_, ItemViewRow>(query.as_str())
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    pub async fn upsert(
        &self,
        id: Uuid,
        path_buf: PathBuf,
        hash: &Hash,
        tags: HashSet<String>,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        let buf = path_buf.canonicalize()?;
        let path_normalized = buf.to_string_lossy();
        let tags_normalized = tags.iter().cloned().collect::<Vec<_>>().join(",");

        let query = format!(
            r#"
            INSERT INTO {0}
            (
                id,
                path,
                hash,
                tags
            )
            VALUES
                (?, ?, ?, ?)
            ON CONFLICT DO UPDATE SET
                path = excluded.path,
                hash = excluded.hash
            "#,
            Self::TABLE_NAME
        );

        sqlx::query(query.as_str())
            .bind(id)
            .bind(&path_normalized)
            .bind(&hash.as_bytes()[..])
            .bind(tags_normalized)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn find_by_hash(
        &self,
        hash: &Hash,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<Option<ItemViewRow>, sqlx::Error> {
        let query = format!(
            r#"
            SELECT *
            FROM {}
            WHERE hash = ?
            "#,
            Self::TABLE_NAME
        );

        let rows = query_as::<_, ItemViewRow>(query.as_str())
            .bind(&hash.as_bytes()[..])
            .fetch_all(executor)
            .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let r = rows.into_iter().next().unwrap();
        Ok(Some(r))
    }

    pub async fn add_tag(
        &self,
        id: Uuid,
        tags: &HashSet<String>,
        executor: impl Executor<'_, Database = Sqlite> + Copy,
    ) -> Result<(), sqlx::Error> {
        let select_sql = format!("SELECT * FROM {} WHERE id = ?", Self::TABLE_NAME);
        let row = sqlx::query_as::<_, ItemViewRow>(&select_sql)
            .bind(id)
            .fetch_optional(executor)
            .await?;

        let Some(row) = row else {
            warn!("No item with id {} in db, cannot add new tags", id);
            return Ok(());
        };

        let mut set: HashSet<String> = if row.tags.is_empty() {
            HashSet::new()
        } else {
            row.tags.split(',').map(str::to_string).collect()
        };

        for t in tags.iter().cloned() {
            set.insert(t);
        }

        let mut merged: Vec<String> = set.into_iter().collect();
        merged.sort_unstable();
        let merged_str = merged.join(",");

        let update_sql = format!("UPDATE {} SET tags = ? WHERE id = ?", Self::TABLE_NAME);
        sqlx::query(&update_sql).bind(&merged_str).bind(id).execute(executor).await?;
        Ok(())
    }

    pub async fn update_path(
        &self,
        id: Uuid,
        new_path: PathBuf,
        executor: impl Executor<'_, Database = Sqlite> + Copy,
    ) -> Result<(), sqlx::Error> {
        let new_path_str = new_path.canonicalize()?.to_string_lossy().to_string();

        let update_sql = format!("UPDATE {} SET path = ? WHERE id = ?", Self::TABLE_NAME);
        sqlx::query(&update_sql)
            .bind(&new_path_str)
            .bind(id)
            .execute(executor)
            .await
            .map(|_| ())
    }

    pub async fn delete(&self, id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<(), sqlx::Error> {
        let query = format!("DELETE FROM {0} WHERE id = ?;", Self::TABLE_NAME);

        sqlx::query(query.as_str()).bind(id).fetch_optional(executor).await.map(|_| ())
    }
}
