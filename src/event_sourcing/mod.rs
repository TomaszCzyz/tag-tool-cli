use crate::DbContext;
use crate::event_sourcing::item_aggregate::{Item, ItemEvent};
use crate::event_sourcing::item_event_handler::ItemEventHandler;
use crate::event_sourcing::sqlite_store::builder::SqliteStoreBuilder;
use crate::event_sourcing::sqlite_store::event_store::SqliteStore;
use crate::event_sourcing::tag_items_event_handler::TagItemsEventHandler;
use color_eyre::Report;
use esrs::manager::AggregateManager;

pub mod item_aggregate;
pub mod item_event_handler;
pub mod item_view;
pub mod sqlite_store;
pub mod tag_items_event_handler;
pub mod tag_items_view;

pub async fn setup_item_store_manager(db_ctx: DbContext) -> color_eyre::Result<AggregateManager<SqliteStore<Item, ItemEvent>>, Report> {
    let item_event_handler = ItemEventHandler {
        pool: db_ctx.db.clone(),
        view: db_ctx.item_view.clone(),
    };

    let tag_items_event_handler = TagItemsEventHandler {
        pool: db_ctx.db.clone(),
        view: db_ctx.tag_items_view.clone(),
    };

    let store: SqliteStore<Item> = SqliteStoreBuilder::new(db_ctx.db.clone())
        .add_event_handler(item_event_handler)
        .add_event_handler(tag_items_event_handler)
        .try_build()
        .await?;

    Ok(AggregateManager::new(store))
}
