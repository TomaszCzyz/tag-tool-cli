use clap::{Parser, Subcommand};

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
    /// Tag an items
    Tag,
}

#[derive(Subcommand)]
pub enum TagsCommands {
    /// Lists all tags.
    List,
    /// Add a new tag
    Add { name: String },
}

#[derive(Subcommand)]
pub enum ItemsCommands {
    /// Search items by tags.
    Search,
}
