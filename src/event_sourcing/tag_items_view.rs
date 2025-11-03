use sqlx::{Executor, Pool, Sqlite};
use tokio::time::Instant;
use tracing::debug;
use uuid::Uuid;

#[allow(unused)]
#[derive(sqlx::FromRow, Debug)]
pub struct TagItemsViewRow {
    pub tag: String,
    pub item_id: Uuid,
}

#[derive(Clone)]
pub struct TagItemsView;

#[allow(dead_code)]
impl TagItemsView {
    const TABLE_NAME: &'static str = "tag_items_view";

    pub async fn new(pool: &Pool<Sqlite>) -> Self {
        let query: String = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {}
            (
                tag     TEXT NOT NULL,
                item_id TEXT NOT NULL,

                PRIMARY KEY (tag, item_id)
            )
            "#,
            Self::TABLE_NAME
        );

        let _ = sqlx::query(query.as_str()).execute(pool).await.unwrap();

        Self
    }

    pub async fn by_id(&self, id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<Option<TagItemsViewRow>, sqlx::Error> {
        let query: String = format!("SELECT * FROM {} WHERE id = ?", Self::TABLE_NAME);

        sqlx::query_as::<_, TagItemsViewRow>(query.as_str())
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    pub async fn handle_tagged(
        &self,
        tag: String,
        item_id: Uuid,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        let query = format!(
            r#"
            INSERT INTO {0}
            (
                tag,
                item_id
            )
            VALUES (?, ?);
            "#,
            Self::TABLE_NAME
        );

        sqlx::query(query.as_str())
            .bind(tag)
            .bind(item_id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn get_all_tags(&self, executor: impl Executor<'_, Database = Sqlite>) -> Result<Vec<String>, sqlx::Error> {
        let start = Instant::now();

        let query = format!(
            r#"
            SELECT DISTINCT tag
            FROM {0}
            ORDER BY tag
            "#,
            Self::TABLE_NAME
        );

        let rows = sqlx::query_as::<_, (String,)>(query.as_str()).fetch_all(executor).await?;

        debug!("get_all_tags: {:?}", start.elapsed());

        Ok(rows.into_iter().map(|(tag,)| tag).collect())
    }

    pub async fn get_by_tags(&self, tags: Vec<String>, executor: impl Executor<'_, Database = Sqlite>) -> Result<Vec<String>, sqlx::Error> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        // Build WHERE clause with OR conditions for each tag
        let placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!(
            r#"
            SELECT DISTINCT iv.path
            FROM {0} tiv
            JOIN item_view iv ON tiv.item_id = iv.id
            WHERE tiv.tag IN ({1})
            ORDER BY iv.path
            "#,
            Self::TABLE_NAME,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, (String,)>(query.as_str());
        for tag in tags {
            query_builder = query_builder.bind(tag);
        }

        let rows = query_builder.fetch_all(executor).await?;
        Ok(rows.into_iter().map(|(path,)| path).collect())
    }

    pub async fn delete(&self, id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<(), sqlx::Error> {
        let query = format!("DELETE FROM {0} WHERE id = $1;", Self::TABLE_NAME);

        sqlx::query(query.as_str()).bind(id).fetch_optional(executor).await.map(|_| ())
    }
}
