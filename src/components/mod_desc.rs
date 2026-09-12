use crate::action::Action;
use crate::components::Component;
use balatui::RemoteMod;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::merge::MergeStrategy;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Wrap};

pub struct ModDesc {
    selected: Option<RemoteMod>,
    has_focus: bool,
}

impl ModDesc {
    pub fn new() -> Self {
        Self {
            selected: None,
            has_focus: false,
        }
    }

    pub fn set_selected(&mut self, selected: Option<RemoteMod>) {
        self.selected = selected;
    }
}

impl Component for ModDesc {
    fn handle_key_event(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Thick)
            .border_style(Style::default().fg(Color::White))
            .merge_borders(MergeStrategy::Exact)
            .padding(Padding::new(5, 5, 1, 1));

        let label_line = |label: &str, value: String| -> Line {
            Line::from(vec![
                Span::styled(format!("{label:<11}"), Style::default().fg(Color::DarkGray)),
                Span::styled(value, Style::default()),
            ])
        };

        let lines: Vec<Line> = match self.selected.as_ref() {
            Some(m) => {
                let mut lines = vec![
                    label_line("Name", m.name.clone()),
                    label_line("Version", m.version.clone()),
                    label_line("By", m.owner.clone()),
                    label_line("Source", m.source.clone()),
                    label_line("Categories", m.categories.join(", ")),
                ];
                let repo = if m.repo.is_empty() {
                    m.package_url.clone()
                } else {
                    m.repo.clone()
                };
                if !repo.is_empty() {
                    lines.push(label_line("Website", repo));
                }
                lines.push(Line::from(""));
                for line in m.description.lines() {
                    lines.push(Line::from(Span::styled(
                        line.to_string(),
                        Style::default().fg(Color::LightBlue),
                    )));
                }
                lines
            }
            None => vec![Line::from(Span::styled(
                "No mods selected",
                Style::default().fg(Color::DarkGray),
            ))],
        };

        frame.render_widget(
            Paragraph::new(lines).block(block).wrap(Wrap { trim: true }),
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
