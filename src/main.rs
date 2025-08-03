mod tags_storage;

use crate::tags_storage::TagsStorage;
use clap::{Arg, Command, command};
use color_eyre::{Report, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};
use std::borrow::Cow;

fn main() -> Result<()> {
    let matches = command!()
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("tags")
                .about("Manage user's tags.")
                .subcommand(Command::new("list").about("Lists all tags."))
                .subcommand(
                    Command::new("add")
                        .arg(Arg::new("name").required(true))
                        .about("Add a new tag"),
                ),
        )
        .subcommand(
            Command::new("items")
                .about("Manage items.")
                .subcommand(Command::new("search").about("Search items by tags.")),
        )
        .subcommand(Command::new("tag").about("Tag an items"))
        .get_matches();

    let mut app = App::new();

    match matches.subcommand() {
        Some(("tag", _)) => launch_tui(app)?,
        Some(("tags", sub_matches)) => match sub_matches.subcommand() {
            Some(("list", _)) => {
                app.tag_storage.list().for_each(|s| println!("{}", s));
                Ok(())
            }
            Some(("add", sub_matches)) => {
                let tag = sub_matches.get_one::<String>("name").unwrap();
                app.tag_storage.add(Cow::Owned(tag.to_string()));
                Ok(())
            }
            _ => unreachable!(
                "Exhausted list of subcommands and subcommand_required prevents `None`"
            ),
        },
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    }?;

    // launch_tui()?
    Ok(())
}

fn launch_tui(app: App) -> Result<Result<()>, Report> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = app.run(terminal);
    ratatui::restore();
    Ok(result)
}

/// The main application which holds the state and logic of the application.
#[derive(Debug, Default)]
pub struct App {
    /// Is the application running?
    running: bool,

    tag_storage: TagsStorage,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let title = Line::from("Ratatui Simple Template")
            .bold()
            .blue()
            .centered();

        let text = "Hello, Ratatui!\n\n\
            Created using https://github.com/ratatui/templates\n\
            Press `Esc`, `Ctrl-C` or `q` to stop running.";

        frame.render_widget(
            Paragraph::new(text)
                .block(Block::bordered().title(title))
                .centered(),
            frame.area(),
        )
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> Result<()> {
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
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.quit(),
            // Add other key handlers here.
            _ => {}
        }
    }

    /// Set running to `false` to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
