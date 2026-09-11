use crate::action::Action;
use crate::components::Component;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};

pub struct About {
    has_focus: bool,
}

impl About {
    pub fn new() -> Self {
        Self { has_focus: false }
    }
}

impl Component for About {
    fn handle_key_event(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from("balatui - a work in progress TUI mod manager for Balatro"),
                Line::from("Requires Lovely and Steamodded to get the most out of it"),
                Line::from("Use Tab/Shift+Tab to move between tabs"),
                Line::from("Quick Options: launch Balatro, open folders, update mods"),
            ])
            .block(
                Block::bordered()
                    .border_type(BorderType::Thick)
                    .merge_borders(MergeStrategy::Exact)
                    .padding(Padding::new(4, 4, 0, 0))
                    .border_style(Style::default().fg(Color::White)),
            ),
            area,
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

