pub mod event_store;
pub mod schema;
pub mod persistable;
pub mod sql;
pub mod builder;

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
