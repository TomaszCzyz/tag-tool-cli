use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, List, Paragraph};
use std::iter;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

/// The main application which holds the state and logic of the application.
pub struct App {
    /// Is the application running?
    running: bool,
    /// Current value of the input box
    input: Input,
    /// History of recorded messages
    messages: Vec<String>,

    tags: Vec<String>,
    top_tag_suggestion: Option<String>,

    matcher: Box<dyn FuzzyMatcher>,
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
            running: false,
            matcher: Box::new(SkimMatcherV2::default()),
            input: Input::default(),
            messages: vec![],
            tags: tags.iter().cloned().collect(),
            top_tag_suggestion: None,
        }
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        loop {
            terminal.draw(|frame| self.render(frame))?;

            let event = event::read()?;
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Enter => {
                        if let Some(tag) = self.top_tag_suggestion.take() {
                            let current_value = self.input.value();
                            let new_value = format!("{} {}", current_value, tag);
                            self.input = Input::from(new_value);
                        }
                        ()
                    }
                    KeyCode::Esc => return Ok(()),
                    _ => {
                        self.input.handle_event(&event);
                        // generate suggestions
                    }
                }
            }
        }
    }

    fn push_message(&mut self) {
        self.messages.push(self.input.value_and_reset());
    }

    fn render(&mut self, frame: &mut Frame) {
        let [header_area, input_area, messages_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(3), Constraint::Min(1)]).areas(frame.area());

        self.render_tags_suggestions(frame, header_area);
        self.render_input(frame, input_area);
        self.render_messages(frame, messages_area);
    }

    fn render_tags_suggestions(&mut self, frame: &mut Frame, area: Rect) {
        let mut score_tags = self
            .tags
            .iter()
            .filter_map(|tag| match self.matcher.fuzzy_indices(tag, self.input.value()) {
                Some(result) => Some((result, tag)),
                None => None,
            })
            .collect::<Vec<_>>();

        score_tags.sort_unstable_by_key(|((s, _), _)| 100 - *s);

        if score_tags.len() > 0 {
            self.top_tag_suggestion = Some(score_tags[0].1.to_string());
        }

        // TODO: calculate number based on number of chars and frame width
        let count = 20;

        let help_message = Line::from_iter(score_tags.into_iter().take(count).flat_map(|((_, indices), tag)| {
            let map = tag.chars().enumerate().map(move |(idx, ch)| {
                let s = ch.to_string();
                if indices.contains(&idx) {
                    Span::styled(s, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else {
                    Span::raw(s)
                }
            });

            Line::from_iter(map.chain(iter::once(Span::raw(" "))))
        }));

        frame.render_widget(help_message, area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let style: Style = Color::Yellow.into();
        let input = Paragraph::new(self.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, area);

        // Ratatui hides the cursor unless it's explicitly set. Position the cursor past the
        // end of the input text and one line down from the border to the input line
        let x = self.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y + 1))
    }

    fn render_messages(&self, frame: &mut Frame, area: Rect) {
        let messages = self.messages.iter().enumerate().map(|(i, message)| format!("{}: {}", i, message));
        let messages = List::new(messages).block(Block::bordered().title("Messages"));
        frame.render_widget(messages, area);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q')) | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to `false` to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
