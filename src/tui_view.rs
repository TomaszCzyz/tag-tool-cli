use crate::tui::Model;
use ratatui::prelude::*;
use ratatui::widgets::{Block, List, Paragraph};
use std::iter;

#[derive(Debug, Default)]
pub struct ViewRenderer;

impl ViewRenderer {
    pub fn render(&self, model: &Model, frame: &mut Frame) {
        let [header_area, input_area, messages_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(3), Constraint::Min(1)]).areas(frame.area());

        self.render_tags_suggestions(model, frame, header_area);
        self.render_input(model, frame, input_area);
        self.render_messages(model, frame, messages_area);
    }

    fn render_tags_suggestions(&self, model: &Model, frame: &mut Frame, area: Rect) {
        let suggestions_line = Line::from_iter(model.tags_suggestions.iter().flat_map(|(tag, indices)| {
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

        frame.render_widget(suggestions_line, area);
    }

    fn render_input(&self, model: &Model, frame: &mut Frame, area: Rect) {
        // keep 2 for borders and 1 for cursor
        let width = area.width.max(3) - 3;
        let scroll = model.input.visual_scroll(width as usize);
        let style: Style = Color::Yellow.into();
        let input = Paragraph::new(model.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Input"));
        frame.render_widget(input, area);

        // Ratatui hides the cursor unless it's explicitly set. Position the cursor past the
        // end of the input text and one line down from the border to the input line
        let x = model.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y + 1))
    }

    fn render_messages(&self, model: &Model, frame: &mut Frame, area: Rect) {
        let messages = model.items.iter().enumerate().map(|(i, message)| format!("{}: {}", i, message));
        let messages = List::new(messages).block(Block::bordered().title("Messages"));
        frame.render_widget(messages, area);
    }
}
