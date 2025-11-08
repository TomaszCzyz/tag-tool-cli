use crate::tag_query::TagQuery;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, propagate_version = true, subcommand_required = true, arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Tigger login flow.
    Login,
    /// Manage user's tags.
    Tags {
        #[command(subcommand)]
        command: TagsCommands,
    },
    /// Search items by tags.
    Search,
    /// Manage items.
    Items {
        #[command(subcommand)]
        command: ItemsCommands,
    },
    /// Tag an item.
    Tag {
        path: PathBuf,
        #[arg(long, value_delimiter = ',', value_name = "TAG[,TAG...]")]
        tags: Option<Vec<String>>,
        #[arg(long = "move-to-common-storage", short = 'm', default_value_t = false)]
        move_to_common_storage: bool,
    },
}

#[derive(Subcommand)]
pub enum TagsCommands {
    /// Lists all tags.
    List,
}

#[derive(Subcommand)]
pub enum ItemsCommands {
    List { tag_query: TagQuery },
}
