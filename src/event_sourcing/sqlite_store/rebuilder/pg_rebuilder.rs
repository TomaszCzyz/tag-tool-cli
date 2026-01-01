use crate::event_sourcing::sqlite_store::SqliteStoreError;
use crate::event_sourcing::sqlite_store::builder::SqliteStoreBuilder;
use crate::event_sourcing::sqlite_store::event_store::SqliteStore;
use crate::event_sourcing::sqlite_store::persistable::Persistable;
use crate::event_sourcing::sqlite_store::rebuilder::Rebuilder;
use crate::event_sourcing::sqlite_store::schema::Schema;
use async_trait::async_trait;
use esrs::Aggregate;
use esrs::handler::{ReplayableEventHandler, TransactionalEventHandler};
use esrs::store::StoreEvent;
use futures::StreamExt;
use sqlx::{Pool, Sqlite, SqliteConnection, Transaction};
use std::marker::PhantomData;

pub struct PgRebuilder<A, Schema = <A as Aggregate>::Event>
where
    A: Aggregate,
{
    event_handlers: Vec<Box<dyn ReplayableEventHandler<A> + Send>>,
    transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, SqliteStoreError, SqliteConnection> + Send>>,
    _schema: PhantomData<Schema>,
}

impl<A> PgRebuilder<A>
where
    A: Aggregate,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_event_handlers(self, event_handlers: Vec<Box<dyn ReplayableEventHandler<A> + Send>>) -> Self {
        Self { event_handlers, ..self }
    }

    pub fn with_transactional_event_handlers(
        self,
        transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, SqliteStoreError, SqliteConnection> + Send>>,
    ) -> Self {
        Self {
            transactional_event_handlers,
            ..self
        }
    }
}

impl<A> Default for PgRebuilder<A>
where
    A: Aggregate,
{
    fn default() -> Self {
        Self {
            event_handlers: vec![],
            transactional_event_handlers: vec![],
            _schema: PhantomData,
        }
    }
}

#[async_trait]
impl<A, S> Rebuilder<A> for PgRebuilder<A, S>
where
    A: Aggregate,
    A::State: Send,
    A::Event: Send + Sync,
    S: Schema<A::Event> + Persistable + Send + Sync,
{
    type Executor = Pool<Sqlite>;
    type Error = SqliteStoreError;

    /// To process all events in the database, a single transaction is opened, and within this
    /// transaction, all aggregates are deleted and for each [`TransactionalEventHandler`], the
    /// events are handled. After the transaction ends, for each [`crate::handler::EventHandler`]
    /// and [`EventBus`], the events are handled.
    async fn all_at_once(&self, pool: Pool<Sqlite>) -> Result<(), Self::Error> {
        let store: SqliteStore<A, _> = SqliteStoreBuilder::new(pool.clone())
            .with_schema::<S>()
            .without_running_migrations()
            .try_build()
            .await?;

        let mut transaction: Transaction<Sqlite> = pool.begin().await?;

        let events: Vec<StoreEvent<A::Event>> = store
            .stream_events(&mut *transaction)
            .collect::<Vec<Result<StoreEvent<A::Event>, Self::Error>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<StoreEvent<A::Event>>, Self::Error>>()?;

        for event in &events {
            for handler in self.transactional_event_handlers.iter() {
                handler.delete(event.aggregate_id, &mut transaction).await?;
                handler.handle(event, &mut transaction).await?;
            }
        }

        transaction.commit().await?;

        for event in &events {
            for handler in self.event_handlers.iter() {
                handler.delete(event.aggregate_id).await;
                handler.handle(event).await;
            }
        }

        Ok(())
    }
}
