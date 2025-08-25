use super::{Component, Eventable};
use balatro_tui::{download_to_tmp, get_balatro_appdata_dir, unzip};
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
use crate::components::optionselector::{Actions, OptionSelector, OptionSelectorText};
use crate::components::textinput::TextInput;
use crate::mods::{ModList, RemoteMod};

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
        installed_mod_selector.title = "Remote mods".to_string();

        let mods_ref = Vec::new();

        let (modlist_tx, modlist_rx) = tokio::sync::mpsc::unbounded_channel();

        installed_mod_selector
            .register_local_action_handler(modlist_tx.clone())
            .expect("Failed to register event handler!");

        let mut searchbar = TextInput::new();

        searchbar.placeholder = "Search...".to_string();
        searchbar.title = "Search".to_string();

        let this = Self {
            action_tx: None,
            has_focus: false,
            options: installed_mod_selector,
            searchbar,
            mods: mods_ref.clone(),
            displayed_mods: mods_ref.clone(),
            local_action_rx: modlist_rx,
            local_action_tx: modlist_tx,
            state: State::Normal,
        };

        this
    }
    pub fn setup_mods(&mut self) {
        self.mods = ModList::get_remote_mods();
        self.displayed_mods = self.mods.clone();
        self.build_options();
    }
    fn build_options(&mut self) {
        self.displayed_mods.sort_by(|a, b| a.title.cmp(&b.title));

        self.options.options.clear();

        self.displayed_mods.iter_mut().for_each(|m| {
            self.options.options.push(
                vec![
                    OptionSelectorText::new(m.title.clone(), Style::default()),
                    OptionSelectorText::new(
                        format!(" {} ", m.version.clone()),
                        Style::default().fg(Color::LightBlue),
                    ),
                    OptionSelectorText::new(
                        format!("by {}", m.author.clone()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    OptionSelectorText::new(
                        format!(" (id: {})", m.identifier.clone()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ], //                Span::styled(format!("{} {} by {:?}", m.name, m.version, m.author), Style::default().fg(Color::Green)),
            );
        });
    }
    fn search(&mut self, query: String) {
        let names: Vec<String> = self
            .mods
            .iter()
            .map(|m| m.title.clone().to_lowercase())
            .collect();
        let all_mods: Vec<&str> = names.iter().map(|s| s.as_str()).collect();

        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());

        let threshold = 0.4f32;
        if query.is_empty() {
            self.displayed_mods = self.mods.clone();
        } else {
            // let res: Vec<(&str, f32)> = fuzzy_search_threshold(&*query, &all_mods, threshold);

            let res = Pattern::parse(&*query, CaseMatching::Ignore, Normalization::Smart)
                .match_list(names, &mut matcher);

            let mut filtered_mods: Vec<RemoteMod> = Vec::new();
            for (m, _) in res {
                let mod_name = m.to_string();
                let mod_opt = self
                    .mods
                    .iter()
                    .find(|m| m.title.to_lowercase() == mod_name);
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
            KeyCode::Char(c) => {
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
                            Actions::Reload => todo!(),
                        }
                    }
                }
                State::Downloading(remote_mod) => {
                    let remote_mod = remote_mod.clone();
                    tokio::spawn(async move {
                        info!(
                            "Now installing {} from {}",
                            remote_mod.title, remote_mod.download_url
                        );

                        let temp_file = download_to_tmp(&*remote_mod.download_url).await;
                        let file = temp_file.as_file();

                        unzip(
                            file,
                            &get_balatro_appdata_dir().join("Mods"),
                            &remote_mod.folder_name,
                        );

                        info!(
                            "Successfully installed {} {}",
                            remote_mod.title, remote_mod.version
                        );
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
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(area);

        // frame.render_widget(
        //     Paragraph::new(Text::from(
        //         Span::styled("Your Input Here", Style::default().fg(Color::Green)),
        //     ))
        //     .block(Block::bordered().border_type(BorderType::Rounded).title("Search")),
        //     vertical_chunks[0]
        // );
        self.searchbar.draw(frame, vertical_chunks[0])?;
        self.options
            .draw(frame, vertical_chunks[1])
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
