use super::Component;
use crate::action::Action;
use crate::components::authoring::AuthoringTools;
use crate::components::modlist::ModlistComponent;
use crate::components::optionselector::{OptionSelector, OptionSelectorText};
use crate::components::quickoptions::QuickOptions;
use crate::components::remotemods::RemoteModsComponent;
use crate::config::Config;
use crate::mods::{is_same_mod, ModList};
use balatro_tui::{fetch_catalog, load_catalog, motd::motd, reinstall_mod, save_catalog, RemoteMod};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use log::{error, info};
use ratatui::{prelude::*, widgets::*};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tui_logger::TuiLoggerWidget;

#[derive(Default)]
enum Focused {
    #[default]
    Modes,
    InstalledMods,
    RemoteMods,
    Authoring,
    Quicks,
}

pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    quick_ops: QuickOptions,
    installed_mod_selector: ModlistComponent,
    remote_mod_selector: RemoteModsComponent,
    mode_selector: OptionSelector,
    focused: Focused,
    authoring: AuthoringTools,
    has_focus: bool,
    catalog_fetched: bool,
    catalog: Vec<RemoteMod>,
}

impl Home {
    pub fn new() -> Self {
        let installed_mod_selector = ModlistComponent::new();

        let mut mode_selector = OptionSelector::new(vec![
            vec![OptionSelectorText::new(
                "Quick Options".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Installed Mods".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Find New Mods".to_string(),
                Style::default(),
            )],
            vec![OptionSelectorText::new(
                "Mod Authoring Tools".to_string(),
                Style::default(),
            )],
        ]);

        mode_selector.has_focus = true;
        mode_selector.title = "Modes".to_string();

        let authoring = AuthoringTools::new();

        let quick_ops = QuickOptions::new();

        let remote_mod_selector = RemoteModsComponent::new();

        Self {
            installed_mod_selector,
            remote_mod_selector,
            mode_selector,
            authoring,
            quick_ops,
            command_tx: None,
            config: Config::default(),
            focused: Focused::Modes,
            has_focus: false,
            catalog_fetched: false,
            catalog: Vec::new(),
        }
    }
}

impl Component for Home {
    fn focus(&mut self) {
        self.has_focus = true;
    }
    fn unfocus(&mut self) {
        self.has_focus = false;
    }
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx.clone());
        self.installed_mod_selector
            .register_action_handler(tx.clone())
            .expect("Failed to register action handler for installed mod selector");
        self.quick_ops
            .register_action_handler(tx.clone())
            .expect("Failed to register action handler for quick ops");
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match self.focused {
            Focused::Modes => {
                match key.code {
                    KeyCode::Right => {
                        match self.mode_selector.selected {
                            0 => {
                                self.focused = Focused::Quicks;
                                self.quick_ops.focus();
                            }
                            1 => {
                                self.focused = Focused::InstalledMods;
                                self.installed_mod_selector.focus();
                            }
                            2 => {
                                self.focused = Focused::RemoteMods;
                                self.remote_mod_selector.focus();
                            }
                            3 => {
                                self.focused = Focused::Authoring;
                                self.authoring.focus();
                            }
                            _ => {}
                        }
                        self.mode_selector.has_focus = false;
                    }
                    _ => {
                        let _ = self.mode_selector.handle_key_event(key);
                    }
                }
            }
            Focused::Quicks => match key.code {
                KeyCode::Left => {
                    self.focused = Focused::Modes;
                    self.quick_ops.unfocus();
                    self.mode_selector.focus();
                }
                _ => {
                    let _ = self.quick_ops.handle_key_event(key);
                }
            },
            Focused::InstalledMods => match key.code {
                KeyCode::Left => {
                    self.focused = Focused::Modes;
                    self.installed_mod_selector.unfocus();
                    self.mode_selector.focus();
                }
                _ => {
                    let _ = self.installed_mod_selector.handle_key_event(key);
                }
            },
            Focused::RemoteMods => match key.code {
                KeyCode::Left => {
                    self.focused = Focused::Modes;
                    self.remote_mod_selector.unfocus();
                    self.mode_selector.focus();
                }
                _ => {
                    let _ = self.remote_mod_selector.handle_key_event(key);
                }
            },
            Focused::Authoring => match key.code {
                KeyCode::Left => {
                    self.focused = Focused::Modes;
                    self.authoring.unfocus();
                    self.mode_selector.focus();
                }
                _ => {
                    let _ = self.authoring.handle_key_event(key);
                }
            },
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                if !self.catalog_fetched {
                    self.catalog_fetched = true;
                    self.catalog = load_catalog();
                    self.remote_mod_selector
                        .update_mods(self.catalog.clone());
                    if let Some(tx) = self.command_tx.clone() {
                        tokio::spawn(async move {
                            info!("Rerolling for mods...");
                            let start = Instant::now();
                            let mods = fetch_catalog().await;
                            let elapsed = start.elapsed();
                            if !mods.is_empty() {
                                save_catalog(&mods);
                            }
                            info!(
                                "Got {} mods in {:.2}s",
                                mods.len(),
                                elapsed.as_secs_f64()
                            );
                            let _ = tx.send(Action::CatalogFetched(mods));
                            tokio::time::sleep(Duration::from_millis(1500)).await;
                            info!("{}", motd());
                        });
                    }
                }
            }
            Action::CatalogFetched(ref mods) => {
                self.catalog = mods.clone();
                self.remote_mod_selector.update_mods(mods.clone());
            }
            Action::ReinstallMods => {
                let catalog = self.catalog.clone();
                if catalog.is_empty() {
                    error!("No mod catalog loaded, refusing to reinstall.");
                } else {
                    tokio::spawn(async move {
                        let installed = ModList::get_local_mods();
                        for m in &installed {
                            match catalog.iter().find(|r| is_same_mod(m, r)) {
                                Some(remote) => {
                                    info!("Reinstalling {}...", remote.title);
                                    let disabled = m.folder.join(".lovelyignore").exists();
                                    if let Err(e) = reinstall_mod(remote, disabled).await {
                                        error!("Failed to reinstall {}: {}", remote.title, e);
                                    }
                                }
                                None => {
                                    info!("{} not in the index, skipping...", m.name);
                                }
                            }
                        }
                        info!("All mods reinstalled.");
                    });
                }
            }
            _ => {}
        }

        self.installed_mod_selector.update(action.clone())?;
        self.quick_ops.update(action.clone())?;
        self.remote_mod_selector.update(action.clone())?;

        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(area);

        frame.render_widget(
            Paragraph::new("Balatro TUI").style(Style::default()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded),
            ),
            vertical_chunks[0],
        );

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(40), Constraint::Min(50)])
            .split(vertical_chunks[1]);
        self.mode_selector.draw(frame, horizontal_chunks[0])?;

        match self.mode_selector.selected {
            0 => {
                self.quick_ops.draw(frame, horizontal_chunks[1])?;
            }
            1 => {
                self.installed_mod_selector
                    .draw(frame, horizontal_chunks[1])?;
            }
            2 => {
                self.remote_mod_selector.draw(frame, horizontal_chunks[1])?;
            }
            3 => {
                self.authoring.draw(frame, horizontal_chunks[1])?;
            }
            _ => {}
        }

        frame.render_widget(
            TuiLoggerWidget::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title("Logs"),
                )
                .output_level(None)
                .style_info(Style::default().fg(Color::LightGreen))
                .style_warn(Style::default().fg(Color::Yellow))
                .style_error(Style::default().fg(Color::Red))
                .style_debug(Style::default().fg(Color::Blue))
                .output_file(false)
                .output_target(false)
                .output_timestamp(None)
                .output_line(false),
            vertical_chunks[2],
        );

        Ok(())
    }
}
