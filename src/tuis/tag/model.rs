use crate::tag::Tag;
use std::path::PathBuf;
use tui_input::Input;

#[derive(Debug, Default)]
pub struct Model {
    pub running_state: RunningState,
    pub editing_mode: EditingMode,
    pub input: Input,
    pub tags: Vec<Tag>,
    pub top_tag_suggestion: Option<Tag>,
    pub file_path: PathBuf,
    /// Tag text with indices of matched characters.
    pub tags_suggestions: Vec<(Tag, Vec<usize>)>,
}

impl Model {
    pub fn input_tags(&self) -> Vec<Tag> {
        self.input
            .value()
            .split_whitespace()
            .filter_map(|s| Tag::try_from(s).ok())
            .collect()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Debug, Default)]
pub enum EditingMode {
    /// We are in the middle of typing a tag name.
    #[default]
    Typing,
    /// The new tag query is empty, we are waiting for the user to start typing
    Idle,
}
