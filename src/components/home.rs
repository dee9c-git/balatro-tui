use super::Component;
use crate::action::Action;
use crate::components::mod_list::ModlistComponent;
use crate::components::option_selector::{OptionSelector, OptionSelectorText};
use crate::components::quick_options::QuickOptions;
use crate::components::remote_mods::RemoteModsComponent;
use crate::config::Config;
use crate::mods::{ModList, is_same_mod};
use balatui::{
    RemoteMod, fetch_catalog, install_dir, load_catalog, motd::motd, reinstall_mod, save_catalog,
};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use log::{error, info};
use ratatui::layout::{Flex, Offset};
use ratatui::{prelude::*, widgets::*};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::UnboundedSender;
use tui_logger::TuiLoggerWidget;

pub struct Home {
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    quick_ops: QuickOptions,
    installed_mod_selector: ModlistComponent,
    remote_mod_selector: RemoteModsComponent,
    mode_selector: OptionSelector,
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
        ]);

        mode_selector.has_focus = true;
        //mode_selector.title = "Modes (Move with Tab/Shift+Tab)".to_string();

        let quick_ops = QuickOptions::new();

        let remote_mod_selector = RemoteModsComponent::new();

        let mut this = Self {
            installed_mod_selector,
            remote_mod_selector,
            mode_selector,
            quick_ops,
            command_tx: None,
            config: Config::default(),
            has_focus: false,
            catalog_fetched: false,
            catalog: Vec::new(),
        };
        this.focus_selected();
        this
    }

    fn select_mode(&mut self, down: bool) {
        self.unfocus_selected();
        let len = self.mode_selector.options.len();
        if len > 1 {
            self.mode_selector.selected = if down {
                (self.mode_selector.selected + 1) % len
            } else {
                (self.mode_selector.selected + len - 1) % len
            };
        }
        self.focus_selected();
    }

    fn focus_selected(&mut self) {
        match self.mode_selector.selected {
            0 => self.quick_ops.focus(),
            1 => self.installed_mod_selector.focus(),
            2 => self.remote_mod_selector.focus(),
            _ => {}
        }
    }

    fn unfocus_selected(&mut self) {
        match self.mode_selector.selected {
            0 => self.quick_ops.unfocus(),
            1 => self.installed_mod_selector.unfocus(),
            2 => self.remote_mod_selector.unfocus(),
            _ => {}
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
        if key.code == KeyCode::Esc {
            return Ok(None);
        }
        match key.code {
            KeyCode::Tab => {
                self.select_mode(true);
            }
            KeyCode::BackTab => {
                self.select_mode(false);
            }
            _ => match self.mode_selector.selected {
                0 => {
                    let _ = self.quick_ops.handle_key_event(key);
                }
                1 => {
                    let _ = self.installed_mod_selector.handle_key_event(key);
                }
                2 => {
                    let _ = self.remote_mod_selector.handle_key_event(key);
                }
                _ => {}
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
                    self.remote_mod_selector.update_mods(self.catalog.clone());
                    if let Some(tx) = self.command_tx.clone() {
                        tokio::spawn(async move {
                            info!("Rerolling for mods...");
                            let start = Instant::now();
                            let mods = fetch_catalog().await;
                            let elapsed = start.elapsed();
                            if !mods.is_empty() {
                                save_catalog(&mods);
                            }
                            info!("Got {} mods in {:.2}s", mods.len(), elapsed.as_secs_f64());
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
                        let mods_dir = ModList::get_local_mod_dir();
                        for m in &installed {
                            match catalog.iter().find(|r| is_same_mod(m, r)) {
                                Some(remote) => {
                                    info!("Reinstalling {}...", remote.name);
                                    let disabled = m.folder.join(".lovelyignore").exists();
                                    let target = install_dir(remote);
                                    let migrating = m.folder != mods_dir.join(&target);
                                    match reinstall_mod(remote, disabled, &target).await {
                                        Ok(_) => {
                                            if migrating {
                                                match std::fs::remove_dir_all(&m.folder) {
                                                    Ok(_) => info!(
                                                        "Migrated {} to {}",
                                                        m.folder.display(),
                                                        target
                                                    ),
                                                    Err(e) => error!(
                                                        "Failed to remove old folder {}: {}",
                                                        m.folder.display(),
                                                        e
                                                    ),
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to reinstall {}: {}", remote.name, e)
                                        }
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
        let [main, _, bottom_line] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);
        frame.render_widget(
            Block::bordered()
                .fg(Color::White)
                .border_type(BorderType::Thick),
            main,
        );
        let titles: Vec<Line> = self
            .mode_selector
            .options
            .iter()
            .map(|opt| Line::from(opt[0].text.clone()))
            .collect();

        /*
        let content_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(vertical_chunks[1]);
        */

        match self.mode_selector.selected {
            0 => {
                self.quick_ops.draw(frame, main)?;
            }
            1 => {
                self.installed_mod_selector.draw(frame, main)?;
            }
            2 => {
                self.remote_mod_selector.draw(frame, main)?;
            }
            _ => {}
        }
        let [options] = Layout::horizontal([Constraint::Length(54)])
            .flex(Flex::Center)
            .areas(main);
        frame.render_widget(
            Tabs::new(titles)
                .select(self.mode_selector.selected)
                .divider("/")
                .highlight_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
                .padding("  ", "  "),
            options,
        );

        frame.render_widget(
            TuiLoggerWidget::default()
                .block(Block::default().padding(Padding::new(3, 3, 0, 0)))
                .output_level(None)
                .style_info(Style::default().fg(Color::LightGreen))
                .style_warn(Style::default().fg(Color::Yellow))
                .style_error(Style::default().fg(Color::Red))
                .style_debug(Style::default().fg(Color::Blue))
                .output_file(false)
                .output_target(false)
                .output_timestamp(None)
                .output_line(false),
            bottom_line,
        );
        frame.render_widget(Block::default().title(Line::from(">>")), bottom_line);

        let mut keys: Vec<String> = vec![];

        match self.mode_selector.selected {
            0 => {
                keys.push("[Up/Down: Switch Option]".to_string());
                keys.push("[Enter: Select Option]".to_string());
                keys.push("[Tab/Shift+Tab: Switch Tabs]".to_string());
                keys.push("[Esc: Exit]".to_string());
            }
            1 => {
                keys.push("[Up/Down: Select Mod]".to_string());
                keys.push("[Shift+D: Delete Mod]".to_string());
                keys.push("[Enter: Select Option]".to_string());
                keys.push("[Tab/Shift+Tab: Switch Tabs]".to_string());
                keys.push("[Esc: Exit]".to_string());
            }
            2 => {
                keys.push("[Enter: Install Mod]".to_string());
                keys.push("[Tab/Shift+Tab: Switch Tabs]".to_string());
                keys.push("[Esc: Exit]".to_string());
            }
            _ => {}
        }
        frame.render_widget(
            Paragraph::new(Text::from(format!("  {}  ", keys.join("  "))))
                .block(Block::default().style(Style::default().fg(Color::White)))
                .alignment(Alignment::Center),
            bottom_line.offset(Offset { x: 0, y: -2 }),
        );
        Ok(())
    }
}
