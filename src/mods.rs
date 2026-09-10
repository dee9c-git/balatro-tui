use balatui::{get_balatro_appdata_dir, RemoteMod};
use log::error;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct ModList;

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
                                    mod_obj.id = "steamodded".to_string();
                                    mod_obj.version = "1.0.0".to_string();
                                    mod_obj.enabled = Some(mod_obj.get_enabled());
                                    mod_obj.force_enable = true;
                                    mod_obj.author =
                                        vec!["the Steamodded contributors".to_string()];

                                    let f = File::open(path.join("version.lua")).unwrap();
                                    let reader = BufReader::new(f);
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
}

fn normalize_ident(s: &str) -> String {
    s.chars()
        .map(|c| c.to_lowercase())
        .flatten()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

pub fn is_same_mod(local: &Mod, remote: &RemoteMod) -> bool {
    let folder_name = local
        .folder
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .unwrap_or_default();

    let author_matches = local.author.iter().any(|a| {
        remote
            .owner
            .split([',', ';'])
            .map(str::trim)
            .any(|r| r.eq_ignore_ascii_case(a.trim()))
    });

    let slug = remote.identifier.rsplit('@').next().unwrap_or("");

    let folder_exact = !remote.folder_name.is_empty() && folder_name == remote.folder_name;
    let name_author =
        local.name.eq_ignore_ascii_case(&remote.name) && author_matches;
    let id_slug = normalize_ident(&local.id) == normalize_ident(slug);
    let id_exact = !remote.id.is_empty() && normalize_ident(&local.id) == normalize_ident(&remote.id);

    folder_exact || name_author || id_slug || id_exact
}

#[derive(Default, Debug, Deserialize)]
#[serde(default)]
pub struct Mod {
    pub id: String,
    pub name: String,
    #[serde(default)]
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
