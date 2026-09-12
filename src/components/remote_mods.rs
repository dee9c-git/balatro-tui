use super::{Component, Eventable};
use balatui::{download_to_tmp, get_balatro_appdata_dir, install_dir, unzip};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use log::info;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};
use ratatui::layout::Direction;
use ratatui::style::Color;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;

use crate::action::Action;
use crate::components::mod_search::TextInput;
use crate::components::option_selector::{Actions, OptionSelector, OptionSelectorText};
use balatui::RemoteMod;

#[derive(Default)]
enum State {
    #[default]
    Normal,
    Downloading(RemoteMod),
}

pub struct RemoteModsComponent {
    pub action_tx: Option<UnboundedSender<Action>>,
    pub has_focus: bool,
    options: OptionSelector,
    searchbar: TextInput,
    mods: Vec<RemoteMod>,
    displayed_mods: Vec<RemoteMod>,
    local_action_tx: mpsc::UnboundedSender<Actions>,
    local_action_rx: mpsc::UnboundedReceiver<Actions>,
    state: State,
}

impl RemoteModsComponent {
    pub fn new() -> Self {
        let mut installed_mod_selector = OptionSelector::new(vec![]);

        let mods_ref = Vec::new();

        let (modlist_tx, modlist_rx) = tokio::sync::mpsc::unbounded_channel();

        installed_mod_selector
            .register_local_action_handler(modlist_tx.clone())
            .expect("Failed to register event handler!");

        let mut searchbar = TextInput::new();

        searchbar.placeholder = "Search...".to_string();

        Self {
            action_tx: None,
            has_focus: false,
            options: installed_mod_selector,
            searchbar,
            mods: mods_ref.clone(),
            displayed_mods: mods_ref.clone(),
            local_action_rx: modlist_rx,
            local_action_tx: modlist_tx,
            state: State::Normal,
        }
    }
    pub fn update_mods(&mut self, mods: Vec<RemoteMod>) {
        self.mods = mods;
        self.displayed_mods = self.mods.clone();
        self.build_options();
    }
    fn build_options(&mut self) {
        self.displayed_mods.sort_by(|a, b| a.name.cmp(&b.name));

        self.options.options.clear();

        self.displayed_mods.iter_mut().for_each(|m| {
            self.options.options.push(
                vec![
                    OptionSelectorText::new(m.name.clone(), Style::default()),
                    OptionSelectorText::new(
                        format!(" by {}", m.owner.clone()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    OptionSelectorText::new(
                        format!(" {}", m.description.clone()),
                        Style::default().fg(Color::LightBlue),
                    ),
                ], //                Span::styled(format!("{} {} by {:?}", m.name, m.version, m.author), Style::default().fg(Color::Green)),
            );
        });
    }
    fn search(&mut self, query: String) {
        let names: Vec<String> = self
            .mods
            .iter()
            .map(|m| m.name.clone().to_lowercase())
            .collect();
        let _all_mods: Vec<&str> = names.iter().map(|s| s.as_str()).collect();

        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());

        let _threshold = 0.4f32;
        if query.is_empty() {
            self.displayed_mods = self.mods.clone();
        } else {
            // let res: Vec<(&str, f32)> = fuzzy_search_threshold(&*query, &all_mods, threshold);

            let res = Pattern::parse(&*query, CaseMatching::Ignore, Normalization::Smart)
                .match_list(names, &mut matcher);

            let mut filtered_mods: Vec<RemoteMod> = Vec::new();
            for (m, _) in res {
                let mod_name = m.to_string();
                let mod_opt = self.mods.iter().find(|m| m.name.to_lowercase() == mod_name);
                if mod_opt.is_some() {
                    filtered_mods.push(mod_opt.unwrap().clone());
                }
            }
            self.displayed_mods = filtered_mods;
        }

        self.build_options();
    }
}

impl Component for RemoteModsComponent {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.action_tx = Some(tx.clone());
        self.options.register_action_handler(tx.clone())?;
        self.options
            .register_local_action_handler(self.local_action_tx.clone())?;
        Ok(())
    }
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Char(_c) => {
                self.searchbar.handle_key_event(key)?;
                self.search(self.searchbar.text.clone());
                self.options.selected = 0;
                self.options.scroll_offset = 0;
            }
            KeyCode::Backspace => {
                self.searchbar.handle_key_event(key)?;
                self.search(self.searchbar.text.clone());
            }
            KeyCode::Enter => {
                let selected_mod = self.displayed_mods[self.options.selected].clone();
                self.state = State::Downloading(selected_mod);
            }
            _ => {
                self.options.handle_key_event(key)?;
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => match &self.state {
                State::Normal => {
                    let act = self.local_action_rx.try_recv();
                    if act.is_ok() {
                        let a = act?;
                        match a {
                            Actions::Selected(c) => {
                                info!("Selected {}", self.displayed_mods[c].identifier.clone());
                            }
                            Actions::Delete(_) => {}
                            Actions::Reload => todo!(),
                        }
                    }
                }
                State::Downloading(remote_mod) => {
                    let remote_mod = remote_mod.clone();
                    tokio::spawn(async move {
                        info!(
                            "Now installing {} from {}",
                            remote_mod.name, remote_mod.download_url
                        );

                        let temp_file = download_to_tmp(&*remote_mod.download_url).await;
                        let file = temp_file.as_file();

                        match unzip(
                            file,
                            &get_balatro_appdata_dir().join("Mods"),
                            &install_dir(&remote_mod),
                        ) {
                            Ok(_) => info!(
                                "Successfully installed {} {}",
                                remote_mod.name, remote_mod.version
                            ),
                            Err(e) => log::error!("Failed to install {}: {}", remote_mod.name, e),
                        }
                    });

                    self.state = State::Normal;
                }
            },
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area);

        let [_, center, _] = Layout::horizontal([
            Constraint::Length(5),
            Constraint::Min(0),
            Constraint::Length(5),
        ])
        .areas(vertical_chunks[1]);
        self.searchbar.draw(frame, vertical_chunks[0])?;
        self.options
            .draw(frame, center)
            .expect("Options failed to draw!");

        Ok(())
    }

    fn focus(&mut self) {
        self.has_focus = true;
        self.options.focus();
        self.searchbar.focus();
    }

    fn unfocus(&mut self) {
        self.has_focus = false;
        self.options.unfocus();
        self.searchbar.unfocus();
    }
}
