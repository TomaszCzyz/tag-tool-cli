use crate::tui_view::ViewRenderer;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode::{Backspace, Char, Delete, End, Home, Left, Right, Tab};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use std::time::Duration;
use tui_input::Input;
use tui_input::InputRequest::{
    DeleteNextChar, DeletePrevChar, DeletePrevWord, DeleteTillEnd, GoToEnd, GoToNextChar, GoToNextWord, GoToPrevChar, GoToPrevWord,
    GoToStart, InsertChar,
};

/// The main application which holds the state and logic of the application.
pub struct App {
    model: Model,
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
        let tags: Vec<String> = std::fs::read_to_string("test-tags.txt")
            .map(|content| {
                content
                    .replace('\n', "")
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self {
            matcher: Box::new(SkimMatcherV2::default()),
            model: Model {
                running_state: RunningState::Running,
                editing_mode: EditingMode::Typing,
                tags: tags.iter().cloned().collect(),
                tags_suggestions: vec![],
                input_tags: vec![],
                top_tag_suggestion: None,
                items: vec![],
                input: Input::default(),
            },
            view_renderer: ViewRenderer::default(),
        }
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let mut model = Model::default();

        while model.running_state != RunningState::Done {
            terminal.draw(|f| self.view_renderer.render(&model, f))?;

            let mut current_msg = self.handle_event(&model)?;
            while let Some(msg) = current_msg {
                current_msg = Self::update(&mut model, msg);
            }
        }

        Ok(())

        // loop {
        //     let event = event::read()?;
        //     if let Event::Key(key) = event {
        //         match key.code {
        //             KeyCode::Enter => {
        //                 if let Some(tag) = self.model.top_tag_suggestion.take() {
        //                     let current_value = self.input.value();
        //                     let new_value = format!("{} {}", current_value, tag);
        //                     self.input = Input::from(new_value);
        //                 }
        //                 ()
        //             }
        //         }
        //     }
        // }
    }

    fn update(model: &mut Model, msg: Message) -> Option<Message> {
        match msg {
            Message::AcceptTopSuggestion => {
                if let Some(tag) = model.top_tag_suggestion.take() {
                    model.input_tags.push(tag);
                    model.editing_mode = EditingMode::Idle;
                    model.input = Input::new("".to_string());

                    // let last_space_index = model.input.rfind(|ch| ch == ' ');
                    return None; // Some(QueryChanged)
                }

                None
            }
            Message::InputKeyEventEntered(key_event) => {
                let input_req = match (key_event.code, key_event.modifiers) {
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
                };

                if let Some(input_req) = input_req {
                    model.input.handle(input_req);
                }

                None
            }
            Message::Exit => {
                model.running_state = RunningState::Done;
                None
            }
        }
    }

    /// Convert Event to Message
    fn handle_event(&self, _: &Model) -> color_eyre::Result<Option<Message>> {
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return Ok(self.handle_key(key));
                }
            }
        }
        Ok(None)
    }

    fn handle_key(&self, key: KeyEvent) -> Option<Message> {
        match self.model.editing_mode {
            EditingMode::Typing => match key.code {
                KeyCode::Enter => Some(Message::AcceptTopSuggestion),
                KeyCode::Esc => Some(Message::Exit),
                _ => Some(Message::InputKeyEventEntered(key)),
            },
            _ => None,
        }
    }

    fn calc_tags_suggestions(&mut self, model: &Model, frame: &mut Frame, area: Rect) -> Vec<(String, Vec<usize>)> {
        let mut score_tags = self
            .model
            .tags
            .iter()
            .filter_map(|tag| match self.matcher.fuzzy_indices(tag, model.input.value()) {
                Some(result) => Some((result, tag)),
                None => None,
            })
            .collect::<Vec<_>>();

        score_tags.sort_unstable_by_key(|((s, _), _)| 100 - *s);

        if score_tags.len() > 0 {
            self.model.top_tag_suggestion = Some(score_tags[0].1.to_string());
        }

        score_tags
            .into_iter()
            .take(30)
            .map(|((_, indices), tag)| (tag.to_string(), indices))
            .collect()
    }
}
