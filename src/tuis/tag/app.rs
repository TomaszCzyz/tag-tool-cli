use crate::event_sourcing::tag_items_view::TagItemsView;
use crate::tuis::tag::model::{Model, RunningState};
use crate::tuis::tag::view::ViewRenderer;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::KeyCode::{Backspace, Char, Delete, End, Home, Left, Right, Tab};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::prelude::Line;
use ratatui::widgets::Block;
use sqlx::{Pool, Sqlite};
use std::time::Duration;
use tui_input::InputRequest::{
    DeleteNextChar, DeletePrevChar, DeletePrevWord, DeleteTillEnd, GoToEnd, GoToNextChar, GoToNextWord, GoToPrevChar, GoToPrevWord,
    GoToStart, InsertChar,
};
use tui_input::{Input, InputRequest};
use crate::tuis::utils::map_to_input_request;

enum Message {
    InputKeyEventEntered(KeyEvent),
    AcceptTopSuggestion,
    UpdateTagsSuggestions,
    Exit,
}

/// The main application which holds the state and logic of the application.
pub struct App {
    view_renderer: ViewRenderer,
    matcher: Box<dyn FuzzyMatcher>,
    pub tag_items_view: Box<TagItemsView>,
    pub db: Pool<Sqlite>,
}

impl App {
    pub fn from(db: Pool<Sqlite>, tag_items_view: Box<TagItemsView>) -> Self {
        Self {
            matcher: Box::new(SkimMatcherV2::default()),
            view_renderer: ViewRenderer::default(),
            tag_items_view,
            db,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        // terminal.draw(|frame| {
        //     let area = frame.area();
        // 
        //     let block = Block::new().title(Line::from("Progress").centered());
        //     frame.render_widget(block, area);
        // })?;
        // return Ok(());

        let tags = self.tag_items_view.get_all_tags(&self.db).await?;

        let mut model = Model {
            running_state: RunningState::Running,
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
                }

                None
            }
            Message::InputKeyEventEntered(key_event) => {
                if key_event.code == Char(' ') && model.input.value().chars().last() == Some(' ') {
                    return None;
                }
                if let Some(req) = map_to_input_request(key_event) {
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
        match key.code {
            KeyCode::Enter => Some(Message::AcceptTopSuggestion),
            KeyCode::Esc => Some(Message::Exit),
            _ => Some(Message::InputKeyEventEntered(key)),
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
