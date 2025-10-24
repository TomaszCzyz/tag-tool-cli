mod event_store;
mod schema;
mod persistable;
mod sql;
mod builder;

#[derive(thiserror::Error, Debug)]
pub enum SqliteStoreError {
    /// Sql error
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    /// Serialization/deserialization error
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Error while running a TransactionalEventHandler inside of the event store.
    #[error(transparent)]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}
