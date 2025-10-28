use async_trait::async_trait;
use sqlx::{Pool, Sqlite};
use std::collections::HashSet;
use uuid::Uuid;

use crate::event_sourcing::item_aggregate::{Item, ItemEvent};
use crate::event_sourcing::item_view::ItemView;
use esrs::handler::EventHandler;
use esrs::store::StoreEvent;

#[derive(Clone)]
pub struct ItemEventHandler {
    pub pool: Pool<Sqlite>,
    pub view: ItemView,
}

#[async_trait]
impl EventHandler<Item> for ItemEventHandler {
    async fn handle(&self, event: &StoreEvent<ItemEvent>) {
        match &event.payload {
            ItemEvent::ItemCreated { path, hash } => {
                self.view
                    .upsert(event.aggregate_id, path.into(), hash, HashSet::new(), &self.pool)
                    .await
                    .unwrap();
            }
            ItemEvent::Tagged { tags } => self.view.add_tag(event.aggregate_id, tags, &self.pool).await.unwrap(),
            ItemEvent::UnTagged { .. } => (),
        }
    }

    async fn delete(&self, aggregate_id: Uuid) {
        if let Err(e) = self.view.delete(aggregate_id, &self.pool).await {
            eprintln!("Error while deleting view: {:?}", e);
        }
    }
}
