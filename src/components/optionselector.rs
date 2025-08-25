use std::cmp::min;
use std::ops::Add;
use std::time::Instant;

use super::{Component, Eventable};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Color, Modifier};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::Span,
    widgets::Paragraph,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;

pub enum Actions {
    Selected(usize),
    Reload,
}

#[derive(Debug, Clone)]
pub struct OptionSelectorText {
    pub text: String,
    pub style: Style,
}

impl OptionSelectorText {
    pub fn new(text: String, style: Style) -> Self {
        Self { text, style }
    }
}

#[derive(Default)]
pub struct OptionSelector {
    pub app_action_tx: Option<UnboundedSender<Action>>,
    pub options: Vec<Vec<OptionSelectorText>>,
    pub selected: usize,
    pub title: String,
    pub has_focus: bool,
    pub action_tx: Option<UnboundedSender<Actions>>,
    pub scroll_offset: usize,
}

impl Clone for OptionSelector {
    fn clone(&self) -> Self {
        let mut s = Self::new(self.options.clone());
        s.selected = self.selected;
        s.title = self.title.clone();
        s.has_focus = self.has_focus;
        s.app_action_tx = self.app_action_tx.clone();
        s.options = self.options.clone();
        s.scroll_offset = self.scroll_offset;

        s
    }
}

impl OptionSelector {
    pub fn new(ops: Vec<Vec<OptionSelectorText>>) -> Self {
        Self {
            options: ops,
            selected: 0,
            app_action_tx: None,
            title: String::new(),
            has_focus: false,
            action_tx: None,
            scroll_offset: 0,
        }
    }
}

impl Component for OptionSelector {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.app_action_tx = Some(tx);
        Ok(())
    }
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                if self.selected < self.options.len() {
                    self.scroll_offset = self.selected.saturating_sub(5);
                }
            }
            KeyCode::Down => {
                self.selected = min(
                    self.selected.saturating_add(1),
                    (self.options.len().saturating_sub(1)),
                );
                if self.selected > 5 {
                    self.scroll_offset = self.selected.saturating_sub(5);
                }
            }
            KeyCode::Enter => {
                if let Some(tx) = self.action_tx.as_ref() {
                    tx.send(Actions::Selected(self.selected))?;
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {}
            Action::Render => {}
            _ => {}
        };
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let mut op_i = 0;
        let ops: Vec<Line> = self
            .options
            .clone()
            .into_iter()
            .map(|str| {
                op_i += 1;
                let mut lines = vec![];
                /*
                if op_i == self.selected + 1 {
                    lines.push(Span::styled(str[0].text, str[0].style.fg(Color::Green)));
                } else {
                    lines.push(Span::styled(str[0].text, str[0].style));
                }
                */

                for s in str {
                    lines.push(Span::styled(s.text, s.style));
                }

                if lines.len() >= 1 && op_i == self.selected + 1 {
                    lines[0] = lines[0].clone().style(
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    );
                }

                Line::from(lines)
            })
            .collect();
        let content = Paragraph::new(ops)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(Span::from(&self.title))
                    .border_style(if self.has_focus {
                        Style::default().fg(Color::LightCyan)
                    } else {
                        Style::default().fg(Color::White)
                    }),
            )
            .scroll((self.scroll_offset as u16, 0));

        frame.render_widget(content, area);

        Ok(())
    }

    fn focus(&mut self) {
        self.has_focus = true;
    }

    fn unfocus(&mut self) {
        self.has_focus = false;
    }
}

impl Eventable<Actions> for OptionSelector {
    fn register_local_action_handler(&mut self, tx: UnboundedSender<Actions>) -> Result<()> {
        self.action_tx = Some(tx);
        Ok(())
    }
}
