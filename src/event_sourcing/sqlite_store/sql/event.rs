use crate::event_sourcing::sqlite_store::persistable::Persistable;
use crate::event_sourcing::sqlite_store::schema::Schema;
use esrs::store::StoreEvent;
use esrs::types::SequenceNumber;
use serde_json::Value;
use sqlx::types::chrono::{DateTime, Utc};
use std::convert::TryInto;
use uuid::Uuid;

/// Event representation on the event store
#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Debug)]
pub struct DbEvent {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
    pub version: Option<i32>,
}

impl DbEvent {
    pub fn try_into_store_event<E, S>(self) -> Result<Option<StoreEvent<E>>, serde_json::Error>
    where
        S: Schema<E>,
    {
        let payload = serde_json::from_value::<S>(self.payload)?.to_event();

        Ok(match payload {
            None => None,
            Some(payload) => Some(StoreEvent {
                id: self.id,
                aggregate_id: self.aggregate_id,
                payload,
                occurred_on: self.occurred_on,
                sequence_number: self.sequence_number,
                version: self.version,
            }),
        })
    }
}

impl<E: Persistable> TryInto<StoreEvent<E>> for DbEvent {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<StoreEvent<E>, Self::Error> {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            payload: serde_json::from_value::<E>(self.payload)?,
            occurred_on: self.occurred_on,
            sequence_number: self.sequence_number,
            version: self.version,
        })
    }
}
