use crate::config::get_data_dir;
use balatro_tui::get_balatro_appdata_dir;
use git2::Repository;
use log::{error, info};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ModList {
    repo: Option<Repository>,
}

impl ModList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_local_mod_dir() -> PathBuf {
        get_balatro_appdata_dir().join("Mods")
    }

    pub fn get_local_mods() -> Vec<Mod> {
        let mod_path = ModList::get_local_mod_dir();

        let mut mods = vec![];
        if let Some(dir) = std::fs::read_dir(mod_path.clone()).ok() {
            for entry in dir {
                let entry = entry.unwrap();
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }

                let mut found_mod_meta = false;

                for file in std::fs::read_dir(&path).unwrap() {
                    let file = file.unwrap();
                    let filepath = file.path();
                    if !filepath.is_file() {
                        continue;
                    }
                    if let Some(extension) = filepath.extension().and_then(|e| e.to_str()) {
                        if extension != "json" {
                            continue;
                        }
                        if let Some(mut mod_obj) = Mod::from_file(&file.path()) {
                            if mod_obj.id.is_empty() {
                                if mod_obj.name == "Steamodded" {
                                    // HACK as Steamodded does not currently have a full
                                    // mod metadata json.
                                    mod_obj.id = "steamodded".to_string();
                                    mod_obj.version = "1.0.0".to_string();
                                    mod_obj.enabled = Some(mod_obj.get_enabled());
                                    mod_obj.force_enable = true;
                                    mod_obj.author =
                                        vec!["the Steamodded contributors".to_string()];

                                    // get version from version.lua
                                    // version.lua format is literally just
                                    // return "1.0.0~BETA-0614a-STEAMODDED"

                                    let f = File::open(path.join("version.lua")).unwrap();
                                    let reader = BufReader::new(f);
                                    // This feels bad to be doing *but* it works.
                                    mod_obj.version = reader
                                        .lines()
                                        .nth(0)
                                        .unwrap()
                                        .unwrap()
                                        .split("\"")
                                        .nth(1)
                                        .unwrap()
                                        .to_string();

                                    mods.push(mod_obj);
                                    found_mod_meta = true;
                                }
                            } else {
                                mod_obj.enabled = Some(mod_obj.get_enabled());
                                mods.push(mod_obj);
                                found_mod_meta = true;
                            }
                        }
                    }
                }

                if !found_mod_meta {
                    let name = path.file_name().unwrap().to_str().unwrap().to_string();
                    if name.to_lowercase().starts_with("lovely")
                        || name.to_lowercase().starts_with("steamodded")
                    {
                        continue;
                    }
                    let mut mod_obj = Mod::new();
                    mod_obj.folder = path.clone();
                    mod_obj.enabled = Some(mod_obj.get_enabled());
                    mod_obj.author = vec!["unknown".to_string()];
                    mod_obj.name = name;
                    mod_obj.version = "(unknown)".to_string();
                    mods.push(mod_obj);
                }
            }
        }
        mods
    }

    pub fn get_remote_mods() -> Vec<RemoteMod> {
        let mut mods = vec![];

        let mut mods_dir = get_data_dir();
        mods_dir.extend(
            ["mods", "mods"], // mods folder inside the repository
        );

        if let Some(dir) = std::fs::read_dir(mods_dir.clone()).ok() {
            for entry in dir {
                let entry = entry.unwrap();
                let path = entry.path();

                if !path.is_dir() {
                    continue;
                }
                if let Some(mod_obj) = RemoteMod::from_directory(&path) {
                    mods.push(mod_obj);
                }
            }
        }

        mods
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct Mod {
    pub id: String,
    pub name: String,
    #[serde(default)] // Not in JSON
    pub folder: PathBuf,
    pub description: String,
    pub version: String,

    #[serde(default)]
    pub author: Vec<String>,

    pub dependencies: Vec<String>,

    #[serde(default)]
    pub enabled: Option<bool>,

    pub force_enable: bool,
}

impl Mod {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_file(path: &Path) -> Option<Self> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);

        let mut loaded_mod: Mod = serde_json::from_reader(reader).ok()?;
        loaded_mod.folder = path.parent()?.to_path_buf();

        Some(loaded_mod)
    }

    pub fn from_directory(path: &Path) -> Option<Self> {
        let mut found_mod = Mod::new();

        for file in std::fs::read_dir(&path).unwrap() {
            let file = file.unwrap();
            let filepath = file.path();
            if !filepath.is_file() {
                continue;
            }
            if let Some(extension) = filepath.extension().and_then(|e| e.to_str()) {
                if extension != "json" {
                    continue;
                }
                if let Some(mut mod_obj) = Mod::from_file(&file.path()) {
                    if mod_obj.id.is_empty() {
                        continue;
                    }
                    mod_obj.enabled = Some(mod_obj.get_enabled());
                    found_mod = mod_obj;
                }
            }
        }

        Some(Self {
            name: found_mod.name,
            id: found_mod.id,
            folder: found_mod.folder,
            description: found_mod.description,
            version: found_mod.version,
            author: found_mod.author,
            dependencies: found_mod.dependencies,
            enabled: found_mod.enabled,
            force_enable: found_mod.force_enable,
        })
    }

    pub fn get_enabled(&mut self) -> bool {
        let mut enabled = true;
        for entry in std::fs::read_dir(&self.folder).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if let Some(name) = path.file_name().and_then(|e| e.to_str()) {
                if name == ".lovelyignore" {
                    enabled = false;
                    break;
                }
            }
        }

        enabled
    }

    pub fn toggle_enabled(&mut self) -> () {
        if self.force_enable {
            error!("This mod is marked as force enabled!");
            return;
        }
        let enabled = self.get_enabled();
        if enabled {
            std::fs::File::create(&self.folder.join(".lovelyignore")).unwrap();
            self.enabled = Some(false);
        } else {
            std::fs::remove_file(&self.folder.join(".lovelyignore"))
                .expect("Failed to remove .lovelyignore file");
            self.enabled = Some(true);
        }
    }
}

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
#[derive(Clone)]
pub struct RemoteMod {
    pub title: String,
    pub version: String,
    pub author: String,
    pub categories: Vec<String>,
    pub repo: String,
    #[serde(rename = "downloadURL")]
    pub download_url: String,
    #[serde(rename = "folderName")]
    pub folder_name: String,
    pub identifier: String,
}

impl RemoteMod {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_json(json: &Value) -> Option<Self> {
        let result = serde_json::from_value(json.clone());
        if let Err(e) = &result {
            println!("Error parsing JSON: {}", e);
        }
        Some(result.ok()?)
    }

    pub fn from_directory(path: &Path) -> Option<Self> {
        let mut found_mod = RemoteMod::new();

        for file in std::fs::read_dir(&path).unwrap() {
            let file = file.unwrap();
            let filepath = file.path();
            if !filepath.is_file() {
                continue;
            }
            if let Some(filename) = filepath.file_name().and_then(|e| e.to_str()) {
                if filename == "meta.json" {
                    let reader = BufReader::new(File::open(&filepath).unwrap());
                    let json: Value = serde_json::from_reader(reader).ok()?;
                    found_mod = RemoteMod::from_json(&json)?;
                }
            }
        }

        found_mod.identifier = path.file_name()?.to_str()?.to_string();

        Some(Self {
            title: found_mod.title.clone(),
            version: found_mod.version.clone(),
            author: found_mod.author,
            categories: found_mod.categories,
            repo: found_mod.repo,
            download_url: found_mod.download_url,
            folder_name: if found_mod.folder_name != "" {
                found_mod.folder_name
            } else {
                found_mod.title
            },
            identifier: found_mod.identifier,
        })
    }
}
