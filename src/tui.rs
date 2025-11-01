use crate::tui_view::ViewRenderer;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode::{Backspace, Char, Delete, End, Home, Left, Right, Tab};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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
}

#[derive(Debug, Default, PartialEq, Eq)]
enum RunningState {
    #[default]
    Running,
    Done,
}

#[derive(Debug, Default)]
pub struct Model {
    running_state: RunningState,
    editing_mode: EditingMode,

    /// Only currently typed text, i.e. part of a single tag
    pub(crate) input: Input,
    /// Accepted tags
    input_tags: Vec<String>,

    tags: Vec<String>,
    top_tag_suggestion: Option<String>,
    /// tag text with indices of matched characters
    pub tags_suggestions: Vec<(String, Vec<usize>)>,
    pub items: Vec<String>,
}

enum Message {
    InputKeyEventEntered(KeyEvent),
    AcceptTopSuggestion,
    UpdateTagsSuggestions,
    Exit,
}

#[derive(Debug, Default)]
enum EditingMode {
    #[default]
    Typing,
    Idle,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self {
            matcher: Box::new(SkimMatcherV2::default()),
            view_renderer: ViewRenderer::default(),
        }
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let tags: Vec<String> = std::fs::read_to_string("test-tags.txt").map(|content| {
            content
                .replace('\n', "")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })?;

        let mut model = Model {
            running_state: RunningState::Running,
            editing_mode: EditingMode::Typing,
            tags: tags.iter().cloned().collect(),
            tags_suggestions: vec![],
            input_tags: vec![],
            top_tag_suggestion: None,
            items: vec![],
            input: Input::default(),
        };

        while model.running_state != RunningState::Done {
            terminal.draw(|f| self.view_renderer.render(&model, f))?;

            let mut current_msg = self.handle_event(&model)?;
            while let Some(msg) = current_msg {
                current_msg = self.update(&mut model, msg);
            }
        }

        Ok(())
    }

    fn update(&mut self, model: &mut Model, msg: Message) -> Option<Message> {
        match msg {
            Message::AcceptTopSuggestion => {
                if let Some(tag) = model.top_tag_suggestion.take() {
                    model.input_tags.push(tag);
                    // model.editing_mode = EditingMode::Idle;
                    model.input = Input::new("".to_string());
                    // return  Some(QueryChanged)
                }

                None
            }
            Message::InputKeyEventEntered(key_event) => {
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
                model.tags_suggestions = self.calc_tags_suggestions(&model.tags, model.input.value());

                if let Some(top_suggestion) = model.tags_suggestions.first() {
                    model.top_tag_suggestion = Some(top_suggestion.0.clone());
                }

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

    fn calc_tags_suggestions(&self, tags: &Vec<String>, pattern: &str) -> Vec<(String, Vec<usize>)> {
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
