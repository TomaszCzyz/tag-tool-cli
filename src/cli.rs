use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    propagate_version = true,
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage user's tags.
    Tags {
        #[command(subcommand)]
        command: TagsCommands,
    },
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
    },
}

#[derive(Subcommand)]
pub enum TagsCommands {
    /// Lists all tags.
    List,
    /// Add a new tag
    Add { names: String },
}

#[derive(Subcommand)]
pub enum ItemsCommands {
    /// Search items by tags.
    Search,
}
