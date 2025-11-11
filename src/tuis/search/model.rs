use crate::tag::Tag;
use crate::tuis::search::view::TaggedItem;
use tui_input::Input;

#[derive(Debug, Default)]
pub struct Model {
    pub running_state: RunningState,
    pub editing_mode: EditingMode,
    /// Only currently typed text, i.e. part of a single tag
    pub input: Input,
    pub tags: Vec<Tag>,
    pub top_tag_suggestion: Option<Tag>,
    /// tag text with indices of matched characters
    pub tags_suggestions: Vec<(Tag, Vec<usize>)>,
    pub tagged_items: Vec<TaggedItem>,
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
    #[default]
    Typing,
    Idle,
}
