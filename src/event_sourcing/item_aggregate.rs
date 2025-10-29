use blake3::Hash;
use esrs::Aggregate;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

pub struct Item;

impl Aggregate for Item {
    const NAME: &'static str = "item";
    type State = ItemState;
    type Command = ItemCommand;
    type Event = ItemEvent;
    type Error = ItemError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            ItemCommand::CreateItem { path, hash } => Ok(vec![ItemEvent::ItemCreated { path, hash }]),
            ItemCommand::Tag { tags } => Ok(vec![ItemEvent::Tagged { tags }]),
            ItemCommand::UnTag { tags } => Ok(vec![ItemEvent::UnTagged { tags }]),
            ItemCommand::Move { new_path } => Ok(vec![ItemEvent::Moved { new_path }]),
        }
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        match payload {
            ItemEvent::ItemCreated { path, hash } => ItemState {
                path,
                hash,
                created_at: Utc::now(),
                tags: state.tags,
            },
            ItemEvent::Tagged { tags } => {
                let mut set = state.tags.clone();
                for tag in tags {
                    set.insert(tag);
                }
                ItemState { tags: set, ..state }
            }
            ItemEvent::UnTagged { tags } => {
                let mut set = state.tags.clone();
                for tag in tags {
                    set.remove(&tag);
                }
                ItemState { tags: set, ..state }
            }
            ItemEvent::Moved { new_path } => ItemState { path: new_path, ..state },
        }
    }
}

#[derive(Debug)]
pub struct ItemState {
    pub path: String,
    pub hash: Hash,
    pub tags: HashSet<String>,
    pub created_at: DateTime<Utc>,
}

impl Default for ItemState {
    fn default() -> Self {
        Self {
            path: "".to_string(),
            hash: Hash::from([0; 32]),
            tags: HashSet::new(),
            created_at: Utc::now(),
        }
    }
}

pub enum ItemCommand {
    CreateItem { path: String, hash: Hash },
    Tag { tags: HashSet<String> },
    UnTag { tags: HashSet<String> },
    Move { new_path: String },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ItemEvent {
    ItemCreated { path: String, hash: Hash },
    Tagged { tags: HashSet<String> },
    UnTagged { tags: HashSet<String> },
    Moved { new_path: String },
}

#[derive(Debug, Error)]
pub enum ItemError {
    ItemNotFound,
}

impl Display for ItemError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
