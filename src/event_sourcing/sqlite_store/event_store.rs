use crate::event_sourcing::sqlite_store::SqliteStoreError;
use crate::event_sourcing::sqlite_store::persistable::Persistable;
use crate::event_sourcing::sqlite_store::schema::Schema;
use crate::event_sourcing::sqlite_store::sql::event::DbEvent;
use crate::event_sourcing::sqlite_store::sql::statements::{Statements, StatementsHandler};
use async_trait::async_trait;
use dashmap::DashMap;
use esrs::bus::EventBus;
use esrs::handler::{EventHandler, TransactionalEventHandler};
use esrs::store::{EventStore, EventStoreLockGuard, StoreEvent, UnlockOnDrop};
use esrs::types::SequenceNumber;
use esrs::{Aggregate, AggregateState};
use futures::StreamExt;
use futures::stream::BoxStream;
use sqlx::types::Json;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{Executor, Pool, Sqlite, SqliteConnection, Transaction};
use std::hash::Hasher;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedMutexGuard, RwLock};
use uuid::Uuid;

pub struct SqliteStore<A, Schema = <A as Aggregate>::Event>
where
    A: Aggregate,
{
    pub(super) inner: Arc<InnerSqliteStore<A>>,
    pub(super) _schema: PhantomData<Schema>,
}

pub(super) struct InnerSqliteStore<A>
where
    A: Aggregate,
{
    pub(super) pool: Pool<Sqlite>,
    pub(super) statements: Statements,
    pub(super) event_handlers: RwLock<Vec<Box<dyn EventHandler<A> + Send>>>,
    pub(super) transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, SqliteStoreError, SqliteConnection> + Send>>,
    pub(super) event_buses: Vec<Box<dyn EventBus<A> + Send>>,
}

impl<A, S> SqliteStore<A, S>
where
    A: Aggregate,
    A::Event: Send + Sync,
    S: Schema<A::Event> + Persistable + Send + Sync,
{
    /// Safely add an event handler to [`Sqlite`]. Since it appends an event handler to a [`RwLock`],
    /// this function needs to be `async`.
    ///
    /// This is mostly used while there's the need to have an event handler that tries to apply a command
    /// on the same aggregate (implementing a saga pattern with event sourcing).
    #[allow(dead_code)]
    pub async fn add_event_handler(&self, event_handler: impl EventHandler<A> + Send + 'static) {
        let mut guard = self.inner.event_handlers.write().await;

        guard.push(Box::new(event_handler))
    }

    /// Save an event in the event store and return a new [`StoreEvent`] instance.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the insert of the values into the database fails.
    pub(crate) async fn save_event(
        &self,
        aggregate_id: Uuid,
        event: A::Event,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<StoreEvent<A::Event>, SqliteStoreError> {
        let id = Uuid::now_v7();
        let version: Option<i32> = None;
        let schema = S::from_event(event);

        let _ = sqlx::query(self.inner.statements.insert())
            .bind(id)
            .bind(aggregate_id)
            .bind(Json(&schema))
            .bind(occurred_on)
            .bind(sequence_number)
            .bind(version)
            .execute(executor)
            .await?;

        Ok(StoreEvent {
            id,
            aggregate_id,
            payload: schema.to_event().expect(
                "For any type that implements Schema the following contract should be upheld:\
                assert_eq!(Some(event.clone()), Schema::from_event(event).to_event())",
            ),
            occurred_on,
            sequence_number,
            version,
        })
    }

    /// This function returns a stream representing the full event store table content. This should
    /// be mainly used to rebuild read models.
    pub fn stream_events<'s>(
        &'s self,
        executor: impl Executor<'s, Database = Sqlite> + 's,
    ) -> BoxStream<'s, Result<StoreEvent<A::Event>, SqliteStoreError>> {
        Box::pin({
            sqlx::query_as::<_, DbEvent>(self.inner.statements.select_all())
                .fetch(executor)
                .map(|res| Ok(res?.try_into_store_event::<_, S>()?))
                .map(Result::transpose)
                .filter_map(std::future::ready)
        })
    }
}

// Global per-aggregate lock registry.
// Use once_cell or lazy_static to init a single map per process.
static LOCKS: once_cell::sync::Lazy<DashMap<u64, Arc<Mutex<()>>>> = once_cell::sync::Lazy::new(|| DashMap::new());

// Hash Uuid to an u64 key (stable enough for the process); or use Uuid as key directly.
fn key_from_uuid(id: Uuid) -> u64 {
    struct H(u64);
    impl Hasher for H {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.0 = self.0.wrapping_mul(1099511628211).wrapping_add(b as u64);
            }
        }
    }
    let mut h = H(1469598103934665603);
    h.write(id.as_bytes());
    h.finish()
}

// Guard that owns the mutex guard. Drop releases the lock.
pub struct InProcessAggregateLockGuard {
    _guard: OwnedMutexGuard<()>,
}

impl UnlockOnDrop for InProcessAggregateLockGuard {}

async fn get_lock_arc(key: u64) -> Arc<Mutex<()>> {
    if let Some(found) = LOCKS.get(&key) {
        return Arc::clone(&*found);
    }
    let new = Arc::new(Mutex::new(()));
    let entry = LOCKS.entry(key).or_insert_with(|| Arc::clone(&new));
    Arc::clone(&*entry)
}

#[async_trait]
impl<A, S> EventStore for SqliteStore<A, S>
where
    A: Aggregate,
    A::State: Send,
    A::Event: Send + Sync,
    S: Schema<A::Event> + Persistable + Send + Sync,
{
    type Aggregate = A;
    type Error = SqliteStoreError;

    async fn lock(&self, aggregate_id: Uuid) -> Result<EventStoreLockGuard, Self::Error> {
        let key = key_from_uuid(aggregate_id);
        let arc = get_lock_arc(key).await;
        let guard = arc.clone().lock_owned().await; // owned guard decouples from &Arc lifetime
        let lock_guard = InProcessAggregateLockGuard { _guard: guard };
        Ok(EventStoreLockGuard::new(lock_guard))
    }

    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<A::Event>>, Self::Error> {
        Ok(sqlx::query_as::<_, DbEvent>(self.inner.statements.by_aggregate_id())
            .bind(aggregate_id)
            .fetch_all(&self.inner.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into_store_event::<_, S>()?))
            .filter_map(Result::transpose)
            .collect::<Result<Vec<StoreEvent<A::Event>>, Self::Error>>()?)
    }

    // Clippy introduced `blocks_in_conditions` lint. With certain version of rust and tracing this
    // line throws an error see: https://github.com/rust-lang/rust-clippy/issues/12281
    #[tracing::instrument(skip_all, fields(aggregate_id = % aggregate_state.id()), err)]
    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<A::State>,
        events: Vec<A::Event>,
    ) -> Result<Vec<StoreEvent<A::Event>>, Self::Error> {
        let mut transaction: Transaction<Sqlite> = self.inner.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<A::Event>> = vec![];

        let aggregate_id = *aggregate_state.id();

        for event in events.into_iter() {
            let store_event: StoreEvent<<A as Aggregate>::Event> = self
                .save_event(
                    aggregate_id,
                    event,
                    occurred_on,
                    aggregate_state.next_sequence_number(),
                    &mut *transaction,
                )
                .await?;

            store_events.push(store_event);
        }

        for store_event in &store_events {
            for transactional_event_handler in &self.inner.transactional_event_handlers {
                let span = tracing::trace_span!(
                    "esrs.transactional_event_handler",
                    event_id = %store_event.id,
                    aggregate_id = %store_event.aggregate_id,
                    transactional_event_handler = transactional_event_handler.name()
                );
                let _e = span.enter();

                if let Err(error) = transactional_event_handler.handle(store_event, &mut transaction).await {
                    tracing::error!({
                        event_id = %store_event.id,
                        aggregate_id = %store_event.aggregate_id,
                        transactional_event_handler = transactional_event_handler.name(),
                        error = ?error,
                    }, "transactional event handler failed to handle event");

                    return Err(error);
                }
            }
        }

        transaction.commit().await?;

        // We need to drop the lock on the aggregate state here as:
        // 1. the events have already been persisted, hence the DB has the latest aggregate;
        // 2. the event handlers below might need to access this aggregate atomically (causing a deadlock!).
        drop(aggregate_state.take_lock());

        let event_handlers = self.inner.event_handlers.read().await;
        for store_event in &store_events {
            for event_handler in event_handlers.iter() {
                let span = tracing::debug_span!(
                    "esrs.event_handler",
                    event_id = %store_event.id,
                    aggregate_id = %store_event.aggregate_id,
                    event_handler = event_handler.name()
                );
                let _e = span.enter();

                event_handler.handle(store_event).await;
            }
        }

        // Publishing to subscribed event buses
        self.publish(&store_events).await;

        Ok(store_events)
    }

    async fn publish(&self, store_events: &[StoreEvent<A::Event>]) {
        let futures: Vec<_> = self
            .inner
            .event_buses
            .iter()
            .map(|bus| async move {
                for store_event in store_events {
                    bus.publish(store_event).await;
                }
            })
            .collect();

        let _ = futures::future::join_all(futures).await;
    }

    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Self::Error> {
        let mut transaction: Transaction<Sqlite> = self.inner.pool.begin().await?;

        let _ = sqlx::query(self.inner.statements.delete_by_aggregate_id())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ())?;

        for transactional_event_handler in self.inner.transactional_event_handlers.iter() {
            transactional_event_handler.delete(aggregate_id, &mut transaction).await?;
        }

        transaction.commit().await?;

        let event_handlers = self.inner.event_handlers.read().await;
        // NOTE: should this be parallelized?
        for event_handler in event_handlers.iter() {
            event_handler.delete(aggregate_id).await;
        }

        Ok(())
    }
}

/// Debug implementation for [`Sqlite`]. It just shows the statements, that are the only thing
/// that might be useful to debug.
impl<T: Aggregate> std::fmt::Debug for SqliteStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sqlite").field("statements", &self.inner.statements).finish()
    }
}

impl<A, S> Clone for SqliteStore<A, S>
where
    A: Aggregate,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _schema: PhantomData,
        }
    }
}
