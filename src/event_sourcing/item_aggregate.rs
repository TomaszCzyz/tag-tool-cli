use esrs::Aggregate;
use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use std::collections::HashSet;
use std::fmt::{Display, Formatter};
use thiserror::Error;

pub struct Item;

impl Aggregate for Item {
    const NAME: &'static str = "item";
    type State = ItemState;
    type Command = ItemCommand;
    type Event = ItemEvent;
    type Error = ItemError;

    fn handle_command(state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            ItemCommand::Tag { tags } => Ok(vec![ItemEvent::Tagged { tags }]),
            ItemCommand::UnTag { tags } => Ok(vec![ItemEvent::UnTagged { tags }]),
        }
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        match payload {
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
        }
    }
}

pub struct ItemState {
    pub path: String,
    pub hash: Box<[u8]>,
    pub tags: HashSet<String>,
    pub created_at: DateTime<Utc>,
}

impl Default for ItemState {
    fn default() -> Self {
        Self {
            path: "".to_string(),
            hash: Box::new([]),
            tags: HashSet::new(),
            created_at: Default::default(),
        }
    }
}

pub enum ItemCommand {
    Tag { tags: Vec<String> },
    UnTag { tags: Vec<String> },
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ItemEvent {
    Tagged { tags: Vec<String> },
    UnTagged { tags: Vec<String> },
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
