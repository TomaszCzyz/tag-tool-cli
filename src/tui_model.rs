use crate::tui_view::TaggedItem;
use tui_input::Input;

#[derive(Debug, Default)]
pub struct Model {
    pub running_state: RunningState,
    pub editing_mode: EditingMode,
    /// Only currently typed text, i.e. part of a single tag
    pub input: Input,
    pub tags: Vec<String>,
    pub top_tag_suggestion: Option<String>,
    /// tag text with indices of matched characters
    pub tags_suggestions: Vec<(String, Vec<usize>)>,
    pub tagged_items: Vec<TaggedItem>,
}

impl Model {
    pub fn input_tags(&self) -> Vec<String> {
        self.input.value().split_whitespace().map(|s| s.to_string()).collect()
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
    #[default]
    Typing,
    Idle,
}
