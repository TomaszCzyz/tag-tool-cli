use async_trait::async_trait;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use crate::event_sourcing::item_aggregate::{Item, ItemEvent};
use crate::event_sourcing::tag_items_view::TagItemsView;
use esrs::handler::EventHandler;
use esrs::store::StoreEvent;

#[derive(Clone)]
pub struct TagItemsEventHandler {
    pub pool: Pool<Sqlite>,
    pub view: TagItemsView,
}

#[async_trait]
impl EventHandler<Item> for TagItemsEventHandler {
    async fn handle(&self, event: &StoreEvent<ItemEvent>) {
        match &event.payload {
            ItemEvent::Tagged { tags } => {
                event.aggregate_id;

                for tag in tags {
                    self.view
                        .handle_tagged(tag.to_string(), event.aggregate_id, &self.pool)
                        .await
                        .expect("failed to handle tagged event");
                }
            }
            _ => (),
        }
    }

    async fn delete(&self, aggregate_id: Uuid) {
        if let Err(e) = self.view.delete(aggregate_id, &self.pool).await {
            eprintln!("Error while deleting view: {:?}", e);
        }
    }
}
