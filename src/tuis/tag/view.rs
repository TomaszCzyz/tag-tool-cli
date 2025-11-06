use crate::tuis::tag::model::Model;
use ratatui::prelude::*;
use ratatui::style::Color;
use ratatui::text::ToSpan;
use std::iter;

#[derive(Debug, Default)]
pub struct ViewRenderer;

impl ViewRenderer {
    pub fn render(&self, model: &Model, frame: &mut Frame) {
        let [header_area, input_area, suggestions_area, _fill_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .areas(frame.area());

        self.render_header(model, frame, header_area);
        self.render_input(model, frame, input_area);
        self.render_tags_suggestions(model, frame, suggestions_area);
    }

    fn render_header(&self, model: &Model, frame: &mut Frame, area: Rect) {
        let file_name = Span::styled("super-file.txt", Style::new().bold().italic());
        let line = Line::from_iter(["Tagging item: ".to_span(), file_name]);

        frame.render_widget(line, area);
    }

    #[allow(unstable_name_collisions)]
    fn render_input(&self, model: &Model, frame: &mut Frame, area: Rect) {
        let width = area.width.max(3) - 3;
        let scroll = model.input.visual_scroll(width as usize);

        let prefix = "New tags: ";
        let input = Line::from_iter([prefix.to_span(), model.input.value().bold()]);

        frame.render_widget(input, area);

        let x = prefix.len() + model.input.visual_cursor().max(scroll) - scroll;
        frame.set_cursor_position((area.x + x as u16, area.y))
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
}
