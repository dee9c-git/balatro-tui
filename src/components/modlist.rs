use super::{Component, Eventable};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use log::info;
use notify::recommended_watcher;
use notify::{Event, RecursiveMode, Watcher};
use ratatui::style::Color;
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, BorderType, Borders};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Style, Stylize},
    text::Span,
    widgets::Paragraph,
};
use std::path::Path;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::optionselector::{Actions, OptionSelector, OptionSelectorText};
use crate::mods;
use crate::mods::{Mod, ModList};

pub struct ModlistComponent {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub has_focus: bool,
    options: OptionSelector,
    mods: Vec<Mod>,
    local_action_tx: mpsc::UnboundedSender<Actions>,
    local_action_rx: mpsc::UnboundedReceiver<Actions>,
}

impl ModlistComponent {
    pub fn new() -> Self {
        let mut installed_mod_selector = OptionSelector::new(vec![]);
        installed_mod_selector.title = "Installed mods".to_string();
        installed_mod_selector.flex = Flex::Center;

        let mods_ref = Vec::new();

        let (modlist_tx, modlist_rx) = tokio::sync::mpsc::unbounded_channel();

        let mut this = Self {
            action_tx: None,
            has_focus: false,
            options: installed_mod_selector,
            mods: mods_ref,
            local_action_rx: modlist_rx,
            local_action_tx: modlist_tx,
        };

        this.mods = ModList::get_local_mods();
        this.build_options();

        let mod_dir = ModList::get_local_mod_dir().as_path().to_path_buf();
        let local_action_tx_clone = this.local_action_tx.clone();

        std::thread::spawn(move || {
            let mut watcher = recommended_watcher(
                move |res: std::result::Result<Event, notify::Error>| match res {
                    Ok(event) => match event {
                        Event {
                            kind: notify::event::EventKind::Create(_),
                            ..
                        }
                        | Event {
                            kind: notify::event::EventKind::Remove(_),
                            ..
                        }
                        | Event {
                            kind: notify::event::EventKind::Modify(_),
                            ..
                        } => {
                            let _ = local_action_tx_clone.send(Actions::Reload);
                        }
                        _ => {}
                    },
                    Err(e) => {
                        log::error!("watch error: {:?}", e);
                    }
                },
            )
            .expect("Failed to create watcher");

            watcher
                .watch(&mod_dir, RecursiveMode::NonRecursive)
                .expect("Failed to watch mod directory");

            // Keep the thread alive
            loop {
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

        this
    }
    fn build_options(&mut self) {
        self.mods.sort_by(|a, b| a.name.cmp(&b.name));

        self.options.options.clear();

        self.mods.iter_mut().for_each(|m| {
            self.options.options.push(
                vec![
                    OptionSelectorText::new(m.name.clone(), Style::default()),
                    OptionSelectorText::new(
                        format!(" {} ", m.version.clone()),
                        Style::default().fg(Color::LightBlue),
                    ),
                    OptionSelectorText::new(
                        format!("by {}", m.author.clone().join(", ")),
                        Style::default().fg(Color::DarkGray),
                    ),
                ], //                Span::styled(format!("{} {} by {:?}", m.name, m.version, m.author), Style::default().fg(Color::Green)),
            );
            if !m.enabled.unwrap_or(true) {
                self.options
                    .options
                    .last_mut()
                    .unwrap()
                    .push(OptionSelectorText::new(
                        " (disabled)".to_string(),
                        Style::default().fg(Color::Red),
                    ));
            }
        });
    }
}

impl Component for ModlistComponent {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx.clone());
        self.options.register_action_handler(tx.clone())?;
        self.options
            .register_local_action_handler(self.local_action_tx.clone())?;
        Ok(())
    }
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            _ => {
                self.options.handle_key_event(key)?;
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                let act = self.local_action_rx.try_recv();
                if act.is_ok() {
                    let a = act?;
                    match a {
                        Actions::Selected(c) => {
                            let m = &mut self.mods[c];
                            m.toggle_enabled();
                            self.build_options();
                        }
                        Actions::Reload => {
                            self.mods = ModList::get_local_mods();
                            self.build_options();

                            // remove any other reload requests from action queue
                            while let Ok(a) = self.local_action_rx.try_recv() {
                                if let Actions::Reload = a {
                                } else {
                                    // put back any other actions
                                    let _ = self.local_action_tx.send(a);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        self.options
            .draw(frame, area)
            .expect("Options failed to draw!");
        Ok(())
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
