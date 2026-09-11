use crate::action::Action;
use crate::components::Component;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Padding, Paragraph};

#[derive(Default)]
pub struct TextInput {
    pub text: String,
    pub placeholder: String,
    pub title: String,
    listening: bool,
    focused: bool,
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(text: String, placeholder: String, title: String) -> Self {
        Self {
            text,
            placeholder,
            title,
            ..Self::default()
        }
    }
}

impl Component for TextInput {
    // fn init(&mut self, area: Size) -> color_eyre::Result<()> {
    //     todo!()
    // }

    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            KeyCode::Char(c) => {
                self.text.push(c);
            }
            KeyCode::Backspace => {
                self.text.pop();
            }

            _ => {}
        }

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        frame.render_widget(
            Paragraph::new(if self.text.is_empty() {
                Line::from(self.placeholder.clone()).style(Style::default().fg(Color::Gray))
            } else {
                Line::from(self.text.clone())
            })
            .block(
                Block::bordered()
                    .border_type(BorderType::Thick)
                    .merge_borders(MergeStrategy::Exact)
                    .padding(Padding::new(4, 4, 0, 0))
                    .border_style(Style::default().fg(Color::White))
                    .title(Line::from(self.title.clone()).centered()),
            ),
            area,
        );

        Ok(())
    }

    fn focus(&mut self) {
        self.focused = true
    }

    fn unfocus(&mut self) {
        self.focused = false
    }
}
