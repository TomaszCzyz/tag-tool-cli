use crate::event_sourcing::item_view::ItemView;
use crate::event_sourcing::tag_items_view::TagItemsView;
use crate::tui_view::ViewRenderer;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode::{Backspace, Char, Delete, End, Home, Left, Right, Tab};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use sqlx::{Pool, Sqlite};
use std::time::Duration;
use tui_input::InputRequest::{
    DeleteNextChar, DeletePrevChar, DeletePrevWord, DeleteTillEnd, GoToEnd, GoToNextChar, GoToNextWord, GoToPrevChar, GoToPrevWord,
    GoToStart, InsertChar,
};
use tui_input::{Input, InputRequest};

/// The main application which holds the state and logic of the application.
pub struct App {
    view_renderer: ViewRenderer,
    matcher: Box<dyn FuzzyMatcher>,
    pub tag_items_view: Box<TagItemsView>,
    pub items_view: Box<ItemView>,
    pub db: Pool<Sqlite>,
}

#[derive(Debug, Default)]
pub struct Model {
    running_state: RunningState,
    editing_mode: EditingMode,
    /// Only currently typed text, i.e. part of a single tag
    pub input: Input,
    tags: Vec<String>,
    top_tag_suggestion: Option<String>,
    /// tag text with indices of matched characters
    pub tags_suggestions: Vec<(String, Vec<usize>)>,
    pub items: Vec<String>,
}

impl Model {
    pub fn input_tags(&self) -> Vec<String> {
        self.input.value().split_whitespace().map(|s| s.to_string()).collect()
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Debug, Default)]
enum EditingMode {
    #[default]
    Typing,
    Idle,
}

enum Message {
    InputKeyEventEntered(KeyEvent),
    QueryChanged,
    AcceptTopSuggestion,
    UpdateTagsSuggestions,
    Exit,
}

impl App {
    pub fn from(db: Pool<Sqlite>, tag_items_view: Box<TagItemsView>, items_view: Box<ItemView>) -> Self {
        Self {
            matcher: Box::new(SkimMatcherV2::default()),
            view_renderer: ViewRenderer::default(),
            tag_items_view,
            items_view,
            db,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let tags = self.tag_items_view.get_all_tags(&self.db).await?;

        let mut model = Model {
            running_state: RunningState::Running,
            editing_mode: EditingMode::Typing,
            tags: tags.iter().cloned().collect(),
            tags_suggestions: vec![],
            top_tag_suggestion: None,
            items: vec![],
            input: Input::default(),
        };

        let mut current_msg = Some(Message::UpdateTagsSuggestions);
        while let Some(msg) = current_msg {
            current_msg = self.update(&mut model, msg).await;
        }

        while model.running_state != RunningState::Done {
            terminal.draw(|f| self.view_renderer.render(&model, f))?;

            let mut current_msg = self.handle_event(&model)?;
            while let Some(msg) = current_msg {
                current_msg = self.update(&mut model, msg).await;
            }
        }

        Ok(())
    }

    async fn update(&mut self, model: &mut Model, msg: Message) -> Option<Message> {
        match msg {
            Message::AcceptTopSuggestion => {
                if let Some(tag) = model.top_tag_suggestion.take() {
                    let t = match model.input.value().rfind(|c: char| c.is_whitespace()) {
                        Some(idx) => format!("{} {} ", &model.input.value()[..idx], tag),
                        None => format!("{} ", tag),
                    };

                    model.input = Input::new(t);
                    model.tags_suggestions = Vec::new();

                    return Some(Message::QueryChanged)
                }

                None
            }
            Message::InputKeyEventEntered(key_event) => {
                if key_event.code == Char(' ') && model.input.value().chars().last() == Some(' ') {
                    return None;
                }
                if let Some(req) = Self::map_to_input_request(key_event) {
                    model.input.handle(req);
                    return Some(Message::UpdateTagsSuggestions);
                }
                None
            }
            Message::Exit => {
                model.running_state = RunningState::Done;
                None
            }
            Message::UpdateTagsSuggestions => {
                let pattern = match model.input.value().rfind(|c: char| c.is_whitespace()) {
                    Some(idx) => &model.input.value()[idx..],
                    None => model.input.value(),
                };
                let input_tags = model.input_tags();
                let vec = model
                    .tags
                    .iter()
                    .filter(|&tag| !input_tags.contains(tag))
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>();
                model.tags_suggestions = self.calc_tags_suggestions(&vec, pattern);

                if let Some(top_suggestion) = model.tags_suggestions.first() {
                    model.top_tag_suggestion = Some(top_suggestion.0.clone());
                }

                None
            }
            Message::QueryChanged => {
                let results = self.tag_items_view.get_by_tags(model.input_tags(), &self.db).await.unwrap();
                model.items = results;
                None
            }
        }
    }

    fn map_to_input_request(key_event: KeyEvent) -> Option<InputRequest> {
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

    /// Convert Event to Message
    fn handle_event(&self, model: &Model) -> color_eyre::Result<Option<Message>> {
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return Ok(self.handle_key(key, model));
                }
            }
        }
        Ok(None)
    }

    fn handle_key(&self, key: KeyEvent, model: &Model) -> Option<Message> {
        match model.editing_mode {
            EditingMode::Typing => match key.code {
                KeyCode::Enter => Some(Message::AcceptTopSuggestion),
                KeyCode::Esc => Some(Message::Exit),
                _ => Some(Message::InputKeyEventEntered(key)),
            },
            _ => None,
        }
    }

    fn calc_tags_suggestions(&self, tags: &[&str], pattern: &str) -> Vec<(String, Vec<usize>)> {
        let pattern = pattern.trim();
        let mut score_tags = tags
            .iter()
            .filter_map(|tag| match self.matcher.fuzzy_indices(tag, pattern) {
                Some(result) => Some((result, tag)),
                None => None,
            })
            .collect::<Vec<_>>();

        score_tags.sort_unstable_by_key(|((s, _), _)| 100 - *s);

        score_tags
            .into_iter()
            .take(30)
            .map(|((_, indices), tag)| (tag.to_string(), indices))
            .collect()
    }
}
