use std::path::PathBuf;

use super::Component;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Direction;
use ratatui::style::Color;
use ratatui::symbols::merge::MergeStrategy;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect, Spacing},
    style::Style,
    text::Span,
    widgets::Paragraph,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::mods::Mod;

#[derive(Debug, Default)]
pub struct AuthoringTools {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub has_focus: bool,
    pub mod_path: PathBuf,
    edited_mod: Mod,
}

impl AuthoringTools {
    pub fn new() -> Self {
        let path = std::env::current_dir().unwrap();
        // let path = PathBuf::from("/home/julie/Documents/GitHub/SnipersTVCorpMod");
        let edited_mod = Mod::from_directory(path.as_path()).unwrap();
        Self {
            mod_path: path,
            edited_mod,
            ..Default::default()
        }
    }
}

impl Component for AuthoringTools {
    fn focus(&mut self) {
        self.has_focus = true;
    }
    fn unfocus(&mut self) {
        self.has_focus = false;
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Up => {}
            KeyCode::Down => {}
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(5)])
            .spacing(Spacing::Overlap(1))
            .split(area);

        if self.edited_mod.id.is_empty() {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::from("No mod was found at "),
                    Span::styled(format!("{}", self.mod_path.display()), Style::default().fg(Color::Yellow)),
                    Span::from(", sorry! Make sure there is a metadata JSON file in the root of the mod directory."),
                ]))
                    .style(Style::default())
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_type(BorderType::Thick)
                            .merge_borders(MergeStrategy::Exact)
                            .border_style(
                                if self.has_focus {
                                    Style::default().fg(Color::White)
                                } else {
                                    Style::default().fg(Color::White)
                                }
                            )
                    ),
            chunks[0]
            );
        } else {
            frame.render_widget(
                //            Paragraph::new(format!("Currently editing mod \'{}\' by {}", self.edited_mod.name, self.edited_mod.author.join(", ")))
                Paragraph::new(Line::from(vec![
                    Span::from("Currently editing mod "),
                    Span::styled(
                        self.edited_mod.name.clone(),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::from(" by "),
                    Span::styled(
                        self.edited_mod.author.join(", "),
                        Style::default().fg(Color::Yellow),
                    ),
                ]))
                .style(Style::default())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Thick)
                        .merge_borders(MergeStrategy::Exact)
                        .border_style(if self.has_focus {
                            Style::default().fg(Color::White)
                        } else {
                            Style::default().fg(Color::White)
                        })
                        .title(
                            Line::from(format!("Editing mod at {}", self.mod_path.display()))
                                .centered(),
                        ),
                ),
                chunks[0],
            );
        }

        Ok(())
    }
}
