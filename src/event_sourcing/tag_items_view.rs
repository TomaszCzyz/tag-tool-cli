use crate::event_sourcing::item_view::ItemViewRow;
use crate::tag::Tag;
use crate::tag_query::TagQuery;
use crate::utils::placeholders;
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
    const TABLE_NAME: &'static str = "view_tags_items";

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

    pub async fn get_all_tags(&self, executor: impl Executor<'_, Database = Sqlite>) -> Result<Vec<Tag>, sqlx::Error> {
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

        Ok(rows.into_iter().filter_map(|(tag,)| Tag::try_from(tag).ok()).collect())
    }

    pub async fn get_by_tags(
        &self,
        tags: &[Tag],
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<Vec<ItemViewRow>, sqlx::Error> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = tags.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let query = format!(
            r#"
            SELECT DISTINCT iv.id, iv.path, iv.hash, iv.tags
            FROM {0} tiv
            JOIN view_items iv ON tiv.item_id = iv.id
            WHERE tiv.tag IN ({1})
            ORDER BY iv.path
            "#,
            Self::TABLE_NAME,
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, ItemViewRow>(query.as_str());
        for tag in tags {
            query_builder = query_builder.bind(tag.as_str());
        }

        let rows = query_builder.fetch_all(executor).await?;
        Ok(rows)
    }

    pub async fn delete(&self, id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<(), sqlx::Error> {
        let query = format!("DELETE FROM {0} WHERE id = $1;", Self::TABLE_NAME);

        sqlx::query(query.as_str()).bind(id).fetch_optional(executor).await.map(|_| ())
    }

    pub async fn list(&self, tag_query: TagQuery, executor: impl Executor<'_, Database = Sqlite>) -> Result<Vec<ItemViewRow>, sqlx::Error> {
        let optional_tags = tag_query.optional_tags();
        let included_tags = tag_query.included_tags();
        let excluded_tags = tag_query.excluded_tags();

        println!("Optional tags: {:?}", optional_tags);
        println!("Included tags: {:?}", included_tags);
        println!("Excluded tags: {:?}", excluded_tags);

        let mut query = r#"
            SELECT DISTINCT iv.id, iv.path, iv.hash, iv.tags
            FROM view_items iv
            WHERE 1=1"#
            .to_string();

        let mut bindings: Vec<String> = Vec::new();

        // Handle included tags (must have ALL of these)
        if !included_tags.is_empty() {
            query.push_str(&format!(
                r#"
                AND iv.id IN (
                    SELECT item_id
                    FROM {0}
                    WHERE tag IN ({1})
                    GROUP BY item_id
                    HAVING COUNT(DISTINCT tag) = ?
                )"#,
                Self::TABLE_NAME,
                placeholders(included_tags.len())
            ));
            bindings.extend(included_tags.iter().map(|t| t.to_string()));
            bindings.push(included_tags.len().to_string());
        }

        // Handle excluded tags (must NOT have ANY of these)
        if !excluded_tags.is_empty() {
            query.push_str(&format!(
                r#"
                AND iv.id NOT IN (
                    SELECT item_id
                    FROM {0}
                    WHERE tag IN ({1})
                )"#,
                Self::TABLE_NAME,
                placeholders(excluded_tags.len())
            ));
            bindings.extend(excluded_tags.iter().map(|t| t.to_string()));
        }

        // Handle optional tags (must have AT LEAST ONE of these)
        if !optional_tags.is_empty() {
            query.push_str(&format!(
                r#"
                AND iv.id IN (
                    SELECT item_id
                    FROM {0}
                    WHERE tag IN ({1})
                )"#,
                Self::TABLE_NAME,
                placeholders(optional_tags.len())
            ));
            bindings.extend(optional_tags.iter().map(|t| t.to_string()));
        }

        query.push_str(
            r#"
            ORDER BY iv.path
            "#,
        );

        println!("Query: {}", query);

        // Build and execute the query
        let mut query_builder = sqlx::query_as::<_, ItemViewRow>(query.as_str());
        for binding in bindings {
            query_builder = query_builder.bind(binding);
        }

        query_builder.fetch_all(executor).await
    }
}
