use balatro_tui::RemoteMod;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    CatalogFetched(Vec<RemoteMod>),
    ReinstallMods,
}
