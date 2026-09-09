pub mod motd;

use log::{error, info};
use platform_dirs::AppDirs;
use reqwest::get;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::process::{Child, Command};
use tempfile::NamedTempFile;

const MOD_CATALOG_URL: &str = "https://bmi-smm.dee9c.com/catalog.json";

pub fn catalog_cache_path() -> PathBuf {
    if let Some(proj_dirs) =
        directories::ProjectDirs::from("net", "sealsearch", env!("CARGO_PKG_NAME"))
    {
        proj_dirs.data_local_dir().join("catalog.json")
    } else {
        PathBuf::from(".").join(".data").join("catalog.json")
    }
}

pub fn save_catalog(mods: &[RemoteMod]) {
    if let Ok(json) = serde_json::to_string_pretty(mods) {
        if let Err(e) = fs::write(catalog_cache_path(), json) {
            error!("Failed to cache catalog: {}", e);
        }
    }
}

pub fn load_catalog() -> Vec<RemoteMod> {
    match fs::read(catalog_cache_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            error!("Failed to parse cached catalog: {}", e);
            vec![]
        }),
        Err(_) => vec![],
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
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

pub async fn fetch_catalog() -> Vec<RemoteMod> {
    match get(MOD_CATALOG_URL).await {
        Ok(resp) => {
            if let Ok(catalog) = resp.json::<Vec<RemoteMod>>().await {
                info!("Fetched {} mods from catalog", catalog.len());
                catalog
            } else {
                error!("Failed to parse mod catalog");
                vec![]
            }
        }
        Err(e) => {
            error!("Failed to fetch mod catalog: {}", e);
            vec![]
        }
    }
}

pub fn launch_balatro(disable_console: bool) -> Result<Child, std::io::Error> {
    #[cfg(unix)]
    use std::os::unix::process::CommandExt;

    let mut cmd = Command::new("steam");
    cmd.arg("-applaunch")
        .arg("2379780")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if disable_console {
        cmd.arg("--disable-console");
    }
    #[cfg(unix)]
    cmd.process_group(0);

    cmd.spawn()
}

pub fn open(path: &str) {
    if let Err(e) = opener::open(path) {
        error!("failed to open: {}", e);
    }
}

pub fn locate_steam_appdata() -> Option<AppDirs> {
    AppDirs::new(Some("Steam"), false)
}

pub fn get_balatro_dir() -> PathBuf {
    let mut path = locate_steam_appdata()
        .expect("failed to locate steam")
        .data_dir;

    path.extend(["steamapps", "common", "Balatro"]);

    path
}

pub fn get_balatro_appdata_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        let steam = locate_steam_appdata().expect("failed to locate steam");
        let mut path = steam.data_dir;
        path.extend([
            "steamapps",
            "compatdata",
            "2379780",
            "pfx",
            "drive_c",
            "users",
            "steamuser",
            "AppData",
            "Roaming",
            "Balatro",
        ]);

        return path;
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        //! UNTESTED
        let balatro = AppDirs::new(Some("Balatro"), false).expect("failed to locate balatro");
        balatro.config_dir
    }
}

pub async fn download_to_tmp(url: &str) -> NamedTempFile {
    let mut tmpfile: NamedTempFile = NamedTempFile::new().unwrap();

    let response = get(url).await.unwrap();

    if response.status().is_success() {
        let content = response.bytes().await.unwrap();

        tmpfile.write_all(&content).unwrap();
    } else {
        panic!("Failed to download file: {:?}", response.status());
    }

    tmpfile
}

pub fn unzip(file: &File, base_path: &PathBuf, dir_name: &str) {
    let mut archive = zip::ZipArchive::new(file).unwrap();

    let target_path = base_path.join(dir_name);

    if target_path.exists() {
        fs::remove_dir_all(&target_path).unwrap();
    }

    fs::create_dir_all(&target_path).unwrap();

    archive.extract(&target_path).unwrap();

    let mut entries = fs::read_dir(&target_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect::<Vec<_>>();

    if entries.len() == 1 && entries[0].path().is_dir() {
        let dir = entries.pop().unwrap().path();

        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let to = target_path.join(entry.file_name());
            fs::rename(&path, &to).expect("failed to rename");
        }

        fs::remove_dir(dir).expect("failed to remove dir");
    }
}

pub async fn install_lovely() {
    info!("Downloading Lovely...");

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        let file = download_to_tmp("https://github.com/ethangreen-dev/lovely-injector/releases/latest/download/lovely-x86_64-pc-windows-msvc.zip").await;

        let target_path = get_balatro_dir().join("version.dll");

        if target_path.exists() {
            fs::remove_file(&target_path).expect("failed to remove existing version.dll");
        }

        let mut archive = zip::ZipArchive::new(file.as_file()).expect("failed to open zip archive");

        let mut file = archive
            .by_name("version.dll")
            .expect("failed to find version.dll in zip archive");

        let mut target_file = File::create(&target_path).expect("failed to create version.dll");
        std::io::copy(&mut file, &mut target_file)
            .expect("failed to copy version.dll to target path");
    }
    #[cfg(target_os = "macos")]
    {
        unimplemented!()
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        info!("Successfully Installed Lovely!")
    }
    #[cfg(target_os = "linux")]
    {
        info!(
            "Successfully Installed Lovely! You may need to set the launch options in Steam to \"WINEDLLOVERRIDES=\"version=n,b\" %command%\""
        );
    }
}
