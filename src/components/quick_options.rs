use crate::action::Action;
use crate::components::option_selector::{Actions, OptionSelector, OptionSelectorText};
use crate::components::{Component, Eventable};
use crate::config::get_data_dir;
use balatui::{get_balatro_appdata_dir, get_balatro_dir, install_lovely, launch_balatro, open};
use crossterm::event::KeyEvent;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::prelude::Color;
use ratatui::style::Style;
use ratatui::symbols::merge::MergeStrategy;
use ratatui::text::Line;
use ratatui::widgets::{Block, BorderType, Paragraph};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

pub struct QuickOptions {
    pub options: OptionSelector,
    pub has_focus: bool,
    action_tx: Option<UnboundedSender<Action>>,
    pub launching_balatro: bool,
    local_action_tx: mpsc::UnboundedSender<Actions>,
    local_action_rx: mpsc::UnboundedReceiver<Actions>,
}

impl QuickOptions {
    pub fn new() -> Self {
        // let mut this = Self {
        //     options: OptionSelector::default(),
        //     has_focus: false,
        //     action_tx: None,
        //     launching_balatro: false,
        // };

        let mut options = OptionSelector::new(vec![
            vec![OptionSelectorText::new(
                "Launch Balatro".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Launch Balatro With Console".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Open Balatro Data Folder".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Open Balatro Mods Folder".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Open Balatro-tui Data Folder".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Install/Update Lovely".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Reinstall/Update All Mods".to_string(),
                Style::default(),
            )],
        ]);

        options.flex = ratatui::layout::Flex::Center;

        // options.title = "Quick Options".to_string();

        let (local_tx, local_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            options,
            has_focus: false,
            action_tx: None,
            launching_balatro: false,
            local_action_tx: local_tx,
            local_action_rx: local_rx,
        }
    }
}

impl Component for QuickOptions {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> color_eyre::Result<()> {
        self.action_tx = Some(tx.clone());
        self.options.register_action_handler(tx.clone())?;
        self.options
            .register_local_action_handler(self.local_action_tx.clone())?;
        Ok(())
    }
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        match key.code {
            _ => {
                if self.launching_balatro {
                    self.launching_balatro = false;
                } else {
                    self.options.handle_key_event(key)?;
                }
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Tick => {
                let act = self.local_action_rx.try_recv();
                if act.is_ok() {
                    let a = act?;
                    match a {
                        Actions::Selected(c) => match c {
                            0 => {
                                launch_balatro(true).expect("Balatro failed to launch!");
                                self.launching_balatro = true;
                            }
                            1 => {
                                launch_balatro(false).expect("Balatro failed to launch!");
                                self.launching_balatro = true;
                            }
                            2 => open(get_balatro_dir().to_str().unwrap()),
                            3 => open(get_balatro_appdata_dir().to_str().unwrap()),
                            4 => open(get_data_dir().to_str().unwrap()),
                            5 => {
                                tokio::spawn(async move {
                                    install_lovely().await;
                                });
                            }
                            6 => {
                                if let Some(tx) = self.action_tx.clone() {
                                    let _ = tx.send(Action::ReinstallMods);
                                }
                            }
                            _ => {}
                        },
                        Actions::Reload => todo!(),
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if !self.launching_balatro {
            self.options.draw(frame, area)
        } else {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::from("Launching Balatro, please wait...")
                        .style(Style::default())
                        .centered(),
                    Line::from("   (Press any key to continue)")
                        .style(Style::default().fg(Color::Gray))
                        .centered(),
                ])
                .block(
                    Block::bordered()
                        .border_type(BorderType::Thick)
                        .border_style(Style::default().fg(Color::White))
                        .merge_borders(MergeStrategy::Exact),
                ),
                area,
            );

            Ok(())
        }
    }

    fn focus(&mut self) {
        self.has_focus = true;
        self.options.focus();
    }

    fn unfocus(&mut self) {
        self.has_focus = false;
        self.options.unfocus();
    }
}
