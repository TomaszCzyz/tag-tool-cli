use ratatui::prelude::*;
use ratatui::style::Color;
use ratatui::widgets::{Block, List, ListItem, Paragraph};
use std::iter;
use std::path::PathBuf;
use crate::tuis::search::model::Model;

#[derive(Debug, Default)]
pub struct ViewRenderer;

impl ViewRenderer {
    pub fn render(&self, model: &Model, frame: &mut Frame) {
        let [header_area, input_area, messages_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(3), Constraint::Min(1)]).areas(frame.area());

        self.render_tags_suggestions(model, frame, header_area);
        self.render_input(model, frame, input_area);
        self.render_items(model, frame, messages_area);
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

    #[allow(unstable_name_collisions)]
    fn render_input(&self, model: &Model, frame: &mut Frame, area: Rect) {
        let width = area.width.max(3) - 3;
        let scroll = model.input.visual_scroll(width as usize);
        let style: Style = Color::Yellow.into();

        let input = Paragraph::new(model.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(Block::bordered().title("Query"));

        frame.render_widget(input, area);

        // Ratatui hides the cursor unless it's explicitly set. Position the cursor past the
        // end of the input text and one line down from the border to the input line
        let x = model.input.visual_cursor().max(scroll) - scroll + 1;
        frame.set_cursor_position((area.x + x as u16, area.y + 1))
    }

    fn render_items(&self, model: &Model, frame: &mut Frame, area: Rect) {
        let items = model
            .tagged_items
            .iter()
            .map(|tagged_item| ListItem::from(tagged_item))
            .collect::<Vec<_>>();

        let messages = List::new(items).block(Block::bordered().title("Items"));
        frame.render_widget(messages, area);
    }
}

#[derive(Debug)]
pub struct TaggedItem {
    pub path: PathBuf,
    pub tags: Vec<String>,
}

impl TaggedItem {
    pub fn new(path: &str, tags: &str) -> Self {
        Self {
            path: PathBuf::from(path),
            tags: tags.split(',').map(|s| s.to_string()).collect(),
        }
    }
}

impl From<&TaggedItem> for ListItem<'_> {
    fn from(item: &TaggedItem) -> Self {
        let t = item.tags.join(" ");
        let file_name = item
            .path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| item.path.to_string_lossy().to_string());

        let line = Line::from_iter([file_name.bold(), Span::raw(" "), t.gray().italic()]);
        ListItem::new(line)
    }
}
