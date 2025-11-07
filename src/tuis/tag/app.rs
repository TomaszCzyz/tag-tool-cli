use crate::DbContext;
use crate::items_tagger::ItemsTagger;
use crate::tuis::tag::model::{EditingMode, Model, RunningState};
use crate::tuis::tag::view::ViewRenderer;
use crate::tuis::utils::{calc_tags_suggestions, map_to_input_request};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode::Char;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use std::path::PathBuf;
use std::time::Duration;
use tui_input::Input;

enum Message {
    InputKeyEventEntered(KeyEvent),
    AcceptTopSuggestion,
    UpdateTagsSuggestions,
    TagAndExit,
    Exit,
}

/// The main application which holds the state and logic of the application.
pub struct TagTui {
    view_renderer: ViewRenderer,
    matcher: Box<dyn FuzzyMatcher>,
    db_ctx: DbContext,
    path_buf: PathBuf,
}

impl TagTui {
    pub async fn from(db_ctx: DbContext, path_buf: PathBuf) -> Self {
        Self {
            matcher: Box::new(SkimMatcherV2::default()),
            view_renderer: ViewRenderer::default(),
            db_ctx,
            path_buf,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        let tags = self.db_ctx.tag_items_view.get_all_tags(&self.db_ctx.db).await?;

        let mut model = Model {
            running_state: RunningState::Running,
            editing_mode: Default::default(),
            tags: tags.iter().cloned().collect(),
            tags_suggestions: vec![],
            top_tag_suggestion: None,
            input: Input::default(),
        };

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
                    model.editing_mode = EditingMode::Idle;
                }

                None
            }
            Message::InputKeyEventEntered(key_event) => {
                if key_event.code == Char(' ') {
                    // Space ends a 'tag search', no matter if the entered tag name is valid or not.
                    model.editing_mode = EditingMode::Idle;
                }

                if key_event.code == Char(' ') && model.input.value().chars().last() == Some(' ') {
                    return None;
                }
                if let Some(req) = map_to_input_request(key_event) {
                    model.editing_mode = EditingMode::Typing;
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
                model.tags_suggestions = calc_tags_suggestions(&vec, pattern, &self.matcher);

                if let Some(top_suggestion) = model.tags_suggestions.first() {
                    model.top_tag_suggestion = Some(top_suggestion.0.clone());
                }

                None
            }
            Message::TagAndExit => {
                let tagger = ItemsTagger::initialize(self.db_ctx.clone()).await;
                tagger.tag_item(&self.path_buf, &model.input_tags(), false).await.unwrap();

                Some(Message::Exit)
            }
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

    fn handle_key(&self, key: KeyEvent, _model: &Model) -> Option<Message> {
        match _model.editing_mode {
            EditingMode::Typing => match key.code {
                KeyCode::Enter => Some(Message::AcceptTopSuggestion),
                KeyCode::Esc => Some(Message::Exit),
                _ => Some(Message::InputKeyEventEntered(key)),
            },
            EditingMode::Idle => match key.code {
                KeyCode::Enter => Some(Message::TagAndExit),
                KeyCode::Esc => Some(Message::Exit),
                _ => None,
            },
        }
    }
}
