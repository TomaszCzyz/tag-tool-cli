use crate::event_sourcing::item_aggregate::{Item, ItemCommand, ItemEvent};
use crate::event_sourcing::sqlite_store::event_store::SqliteStore;
use crate::utils::hash_file;
use crate::{DbContext, USER_DIRS, setup_item_store_manager};
use blake3::Hash;
use color_eyre::Report;
use crossterm::event::{Event, KeyCode, KeyEvent, read};
use esrs::AggregateState;
use esrs::manager::AggregateManager;
use log::info;
use same_file::is_same_file;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tracing::error;
use uuid::Uuid;

pub struct ItemsTagger {
    db_ctx: DbContext,
    manager: AggregateManager<SqliteStore<Item, ItemEvent>>,
}

impl ItemsTagger {
    pub(crate) async fn initialize(db_ctx: DbContext) -> Self {
        let manager = setup_item_store_manager(db_ctx.clone()).await.unwrap();

        Self { db_ctx, manager }
    }

    pub async fn tag_item(&self, path: &PathBuf, tags: &[String], move_to_common_storage: bool) -> color_eyre::Result<(), Report> {
        let db_ctx = &self.db_ctx;
        let manager = &self.manager;

        let hash = hash_file(path);
        let aggregate_id = if let Some(item) = db_ctx.item_view.find_by_hash(&hash, &db_ctx.db).await? {
            info!("Found item: {:}", item);
            // TODO: validate case, when paths are the same, but contents are different
            if let Ok(is_same) = is_same_file(&item.path, path) {
                if !is_same {
                    panic!("File with the same content is already tracked")
                }
            };
            item.id
        } else {
            info!("Creating new item: {:?}", path);
            self.create_item(path, hash).await?
        };
        let aggregate_state = manager.load(aggregate_id).await?.unwrap();

        let mut input_tags = HashSet::from_iter(tags.iter().cloned());
        input_tags.retain(|t| !aggregate_state.inner().tags.contains(t));

        if input_tags.is_empty() {
            info!("The item already has tags: {:?}.", tags);

            if move_to_common_storage {
                self.move_item_to_common_storage(aggregate_id).await?;
            }

            return Ok(());
        }

        manager
            .handle_command(aggregate_state, ItemCommand::Tag { tags: input_tags })
            .await??;

        if move_to_common_storage {
            self.move_item_to_common_storage(aggregate_id).await?;
        }

        Ok(())
    }

    async fn create_item(&self, path: &PathBuf, hash: Hash) -> color_eyre::Result<Uuid, Report> {
        let new_aggregate_id = Uuid::new_v4();
        let aggregate_state = AggregateState::with_id(new_aggregate_id);

        self.manager
            .handle_command(
                aggregate_state,
                ItemCommand::CreateItem {
                    hash,
                    path: path.to_string_lossy().to_string(),
                },
            )
            .await??;

        Ok(new_aggregate_id)
    }

    async fn move_item_to_common_storage(&self, aggregate_id: Uuid) -> color_eyre::Result<(), Report> {
        info!("Moving item to common storage: {:?}", aggregate_id);
        let aggregate_state = self.manager.load(aggregate_id).await?.unwrap();
        if let Some(doc_path) = USER_DIRS.document_dir() {
            let item_path = PathBuf::from(&aggregate_state.inner().path).canonicalize()?;
            let original_file_name = item_path
                .file_name()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "source path has no file name"))?;

            let mut dest_path_buf = doc_path.to_path_buf();
            dest_path_buf.push("tag-tool");
            dest_path_buf.push("common-storage");

            fs::create_dir_all(&dest_path_buf).expect("Failed to create directory");

            dest_path_buf.push(original_file_name);

            if dest_path_buf.exists() {
                println!("File already exists in common storage: {:?}", dest_path_buf);
                println!("Do you want to override it? [y/N]");

                match read()? {
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('y'), ..
                    }) => (),
                    Event::Key(KeyEvent {
                        code: KeyCode::Char('Y'), ..
                    }) => (),
                    _ => return Ok(()),
                }
            }

            match fs::rename(item_path, &dest_path_buf) {
                Ok(_) => {
                    self.manager
                        .handle_command(
                            aggregate_state,
                            ItemCommand::Move {
                                new_path: dest_path_buf.to_string_lossy().to_string(),
                            },
                        )
                        .await??;
                }
                Err(e) => {
                    error!("Failed to move file to common storage: {:?}, error: {}", dest_path_buf, e);
                }
            }
        }
        Ok(())
    }
}
