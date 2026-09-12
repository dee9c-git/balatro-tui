use crate::action::Action;
use crate::components::Component;
use balatui::motd::motd;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Padding, Paragraph};

pub struct About {
    has_focus: bool,
    motd: String,
}

impl About {
    pub fn new() -> Self {
        Self {
            has_focus: false,
            motd: motd(),
        }
    }
}

impl Component for About {
    fn handle_key_event(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        fn make_colored_line(text: &str) -> Line {
            Line::from(Span::styled(text, Style::default().fg(Color::Blue)))
        }
        frame.render_widget(
            Paragraph::new(vec![
                make_colored_line("██▀▀█▄   ▄█▀▀█▄ ██       ▄█▀▀█▄ ▀▀██▀▀ ██   ██ ██"),
                make_colored_line("██  ██  ██   ██ ██      ██   ██   ██   ██   ██ ██"),
                make_colored_line("██▀▀▀█▄ ██   ██ ██      ██   ██   ██   ██   ██ ██"),
                make_colored_line("██   ██ ██▀▀▀██ ██      ██▀▀▀██   ██   ██   ██ ██"),
                make_colored_line("██▄▄█▀  ██   ██ ██▄▄▄▄▄ ██   ██   ██   ▀█▄▄█▀  ██"),
                Line::from(""),
                Line::from(""),
                Line::from(Span::styled(
                    self.motd.to_owned(),
                    Style::default().fg(Color::Green),
                )),
                Line::from(""),
                Line::from("Balatro Mod Manager in the terminal"),
                Line::from("Original balatro-tui by colonthreeing"),
                Line::from("Modified by Dee9c"),
                Line::from(""),
            ])
            .alignment(Alignment::Center)
            .block(Block::default().padding(Padding::new(4, 4, 1, 1))),
            area.centered_vertically(Constraint::Length(16)),
        );

        Ok(())
    }

    fn focus(&mut self) {
        self.has_focus = true;
    }

    fn unfocus(&mut self) {
        self.has_focus = false;
    }
}
