use esrs::Aggregate;
use esrs::bus::EventBus;
use esrs::handler::{EventHandler, TransactionalEventHandler};
use sqlx::{Pool, Sqlite, SqliteConnection};
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::persistable::Persistable;
use crate::event_sourcing::sqlite_store::SqliteStoreError;
use crate::event_sourcing::sqlite_store::event_store::{InnerSqliteStore, SqliteStore};
use crate::event_sourcing::sqlite_store::schema::Schema;
use crate::event_sourcing::sqlite_store::sql::migrations::{Migrations, MigrationsHandler};
use crate::event_sourcing::sqlite_store::sql::statements::{Statements, StatementsHandler};

/// Struct used to build a brand new [`SqliteStore`].
pub struct SqliteStoreBuilder<A, Schema = <A as Aggregate>::Event>
where
    A: Aggregate,
{
    pool: Pool<Sqlite>,
    statements: Statements,
    event_handlers: Vec<Box<dyn EventHandler<A> + Send>>,
    transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, SqliteStoreError, SqliteConnection> + Send>>,
    event_buses: Vec<Box<dyn EventBus<A> + Send>>,
    run_migrations: bool,
    _schema: PhantomData<Schema>,
}

impl<A> SqliteStoreBuilder<A, <A as Aggregate>::Event>
where
    A: Aggregate,
{
    /// Creates a new instance of a [`SqliteStoreBuilder`].
    pub fn new(pool: Pool<Sqlite>) -> SqliteStoreBuilder<A, <A as Aggregate>::Event> {
        SqliteStoreBuilder {
            pool,
            statements: Statements::new::<A>(),
            event_handlers: vec![],
            transactional_event_handlers: vec![],
            event_buses: vec![],
            run_migrations: true,
            _schema: PhantomData,
        }
    }
}

impl<A, S> SqliteStoreBuilder<A, S>
where
    A: Aggregate,
{
    /// Set the event handlers list
    pub fn with_event_handlers(mut self, event_handlers: Vec<Box<dyn EventHandler<A> + Send>>) -> Self {
        self.event_handlers = event_handlers;
        self
    }

    /// Add a single event handler
    pub fn add_event_handler(mut self, event_handler: impl EventHandler<A> + Send + 'static) -> Self {
        self.event_handlers.push(Box::new(event_handler));
        self
    }

    /// Set transactional event handlers list
    pub fn with_transactional_event_handlers(
        mut self,
        transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, SqliteStoreError, SqliteConnection> + Send>>,
    ) -> Self {
        self.transactional_event_handlers = transactional_event_handlers;
        self
    }

    /// Add a single transactional event handler
    pub fn add_transactional_event_handler(
        mut self,
        transaction_event_handler: impl TransactionalEventHandler<A, SqliteStoreError, SqliteConnection> + Send + 'static,
    ) -> Self {
        self.transactional_event_handlers.push(Box::new(transaction_event_handler));
        self
    }

    /// Set event buses list
    pub fn with_event_buses(mut self, event_buses: Vec<Box<dyn EventBus<A> + Send>>) -> Self {
        self.event_buses = event_buses;
        self
    }

    /// Add a single event bus
    pub fn add_event_bus(mut self, event_bus: impl EventBus<A> + Send + 'static) -> Self {
        self.event_buses.push(Box::new(event_bus));
        self
    }

    /// Calling this function the caller avoid running migrations. It is recommend to run migrations
    /// at least once per store per startup.
    pub fn without_running_migrations(mut self) -> Self {
        self.run_migrations = false;
        self
    }

    /// Set the schema of the underlying SqliteStore.
    pub fn with_schema<N>(self) -> SqliteStoreBuilder<A, N>
    where
        N: Schema<A::Event> + Persistable + Send + Sync,
    {
        SqliteStoreBuilder {
            pool: self.pool,
            statements: self.statements,
            run_migrations: self.run_migrations,
            event_handlers: self.event_handlers,
            transactional_event_handlers: self.transactional_event_handlers,
            event_buses: self.event_buses,
            _schema: PhantomData,
        }
    }

    /// This function runs all the needed [`Migrations`], atomically setting up the database if
    /// `run_migrations` isn't explicitly set to false. [`Migrations`] should be run only at application
    /// startup due to avoid performance issues.
    ///
    /// Eventually returns an instance of SqliteStore.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if there's an error running [`Migrations`].
    pub async fn try_build(self) -> Result<SqliteStore<A, S>, sqlx::Error> {
        if self.run_migrations {
            Migrations::run::<A>(&self.pool).await?;
        }

        Ok(SqliteStore {
            inner: Arc::new(InnerSqliteStore {
                pool: self.pool,
                statements: self.statements,
                event_handlers: RwLock::new(self.event_handlers),
                transactional_event_handlers: self.transactional_event_handlers,
                event_buses: self.event_buses,
            }),
            _schema: self._schema,
        })
    }
}
