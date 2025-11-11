use crate::tag::Tag;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::crossterm::event::KeyCode::{Backspace, Char, Delete, End, Home, Left, Right, Tab};
use ratatui::crossterm::event::{KeyEvent, KeyModifiers};
use tui_input::InputRequest;
use tui_input::InputRequest::{
    DeleteNextChar, DeletePrevChar, DeletePrevWord, DeleteTillEnd, GoToEnd, GoToNextChar, GoToNextWord, GoToPrevChar, GoToPrevWord,
    GoToStart, InsertChar,
};

pub fn map_to_input_request(key_event: KeyEvent) -> Option<InputRequest> {
    match (key_event.code, key_event.modifiers) {
        (Backspace, KeyModifiers::NONE) => Some(DeletePrevChar),
        (Delete, KeyModifiers::NONE) => Some(DeleteNextChar),
        (Tab, KeyModifiers::NONE) => None,
        (Left, KeyModifiers::NONE) => Some(GoToPrevChar),
        (Left, KeyModifiers::CONTROL) => Some(GoToPrevWord),
        (Right, KeyModifiers::NONE) => Some(GoToNextChar),
        (Right, KeyModifiers::CONTROL) => Some(GoToNextWord),
        (Char('w'), KeyModifiers::CONTROL) => Some(DeletePrevWord),
        (Char('k'), KeyModifiers::CONTROL) => Some(DeleteTillEnd),
        (Home, KeyModifiers::NONE) => Some(GoToStart),
        (End, KeyModifiers::NONE) => Some(GoToEnd),
        (Char(c), KeyModifiers::NONE) => Some(InsertChar(c)),
        (Char(c), KeyModifiers::SHIFT) => Some(InsertChar(c)),
        (_, _) => None,
    }
}

pub(crate) fn calc_tags_suggestions(tags: &[&Tag], pattern: &str, matcher: &Box<dyn FuzzyMatcher>) -> Vec<(Tag, Vec<usize>)> {
    let pattern = pattern.trim();
    let mut score_tags = tags
        .iter()
        .filter_map(|tag| match matcher.fuzzy_indices(tag, pattern) {
            Some(result) => Some((result, tag)),
            None => None,
        })
        .collect::<Vec<_>>();

    score_tags.sort_unstable_by_key(|((s, _), _)| 100 - *s);

    score_tags
        .into_iter()
        .take(30)
        .map(|((_, indices), &tag)| (Tag::from(tag.clone()), indices))
        .collect()
}
