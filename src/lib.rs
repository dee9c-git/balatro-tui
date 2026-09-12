pub mod motd;

use log::{error, info};
use platform_dirs::AppDirs;
use reqwest::get;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::PathBuf;
use std::process::Stdio;
use std::process::{Child, Command};
use tempfile::NamedTempFile;

const MOD_CATALOG_URL: &str = "https://media.githubusercontent.com/media/frostice482/balatro-mod-index-tiny/master/out.json.gz";
const TS_CATALOG_URL: &str = "https://thunderstore.io/c/balatro/api/v1/package/";

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

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
pub struct RemoteMod {
    pub name: String,
    pub version: String,
    pub owner: String,
    #[serde(default)]
    pub categories: Vec<String>,
    pub repo: String,
    pub download_url: String,
    #[serde(rename = "folderName", default)]
    pub folder_name: String,
    #[serde(rename = "pathname")]
    pub identifier: String,
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_source")]
    pub source: String,
    #[serde(default)]
    pub package_url: String,
}

fn default_source() -> String {
    "bmi".to_string()
}

#[derive(Deserialize)]
struct TsPackage {
    name: String,
    owner: String,
    #[serde(rename = "package_url", default)]
    package_url: String,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    is_deprecated: bool,
    #[serde(default)]
    versions: Vec<TsVersion>,
}

#[derive(Deserialize)]
struct TsVersion {
    #[serde(rename = "version_number")]
    version_number: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    download_url: String,
}

const TS_BLACKLIST: &[&str] = &["r2modman", "lovely"];

pub fn install_dir(remote: &RemoteMod) -> String {
    if remote.folder_name.is_empty() {
        remote.identifier.clone()
    } else {
        remote.folder_name.clone()
    }
}

pub async fn fetch_catalog() -> Vec<RemoteMod> {
    match get(MOD_CATALOG_URL).await {
        Ok(resp) => {
            match resp.bytes().await {
                Ok(bytes) => {
                    let data: Vec<u8> = if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b
                    {
                        let mut out = Vec::new();
                        let mut decoder =
                            flate2::read::GzDecoder::new(std::io::Cursor::new(&bytes[..]));
                        match std::io::Read::read_to_end(&mut decoder, &mut out) {
                            Ok(_) => out,
                            Err(e) => {
                                error!("Failed to decompress mod catalog: {}", e);
                                return vec![];
                            }
                        }
                    } else {
                        bytes.to_vec()
                    };
                    if let Ok(catalog) = serde_json::from_slice::<Vec<RemoteMod>>(&data) {
                        info!("Fetched {} mods from catalog", catalog.len());
                        catalog
                    } else {
                        error!("Failed to parse mod catalog");
                        vec![]
                    }
                }
                Err(e) => {
                    error!("Failed to read mod catalog: {}", e);
                    vec![]
                }
            }
        }
        Err(e) => {
            error!("Failed to fetch mod catalog: {}", e);
            vec![]
        }
    }
}

pub async fn fetch_ts_catalog() -> Vec<RemoteMod> {
    match get(TS_CATALOG_URL).await {
        Ok(resp) => match resp.json::<Vec<TsPackage>>().await {
            Ok(packages) => {
                let mods: Vec<RemoteMod> = packages
                    .iter()
                    .filter(|p| {
                        !p.is_deprecated && !TS_BLACKLIST.contains(&p.name.to_lowercase().as_str())
                    })
                    .filter_map(|p| {
                        let v = p.versions.first()?;
                        Some(RemoteMod {
                            name: p.name.clone(),
                            version: v.version_number.clone(),
                            owner: p.owner.clone(),
                            categories: p.categories.clone(),
                            repo: String::new(),
                            download_url: v.download_url.clone(),
                            folder_name: format!("{}@{}", p.owner, p.name),
                            identifier: p.name.clone(),
                            id: p.name.clone(),
                            description: v.description.clone(),
                            source: "thunderstore".to_string(),
                            package_url: p.package_url.clone(),
                        })
                    })
                    .collect();
                info!("Fetched {} mods from Thunderstore", mods.len());
                mods
            }
            Err(e) => {
                error!("Failed to parse Thunderstore catalog: {}", e);
                vec![]
            }
        },
        Err(e) => {
            error!("Failed to fetch Thunderstore catalog: {}", e);
            vec![]
        }
    }
}

fn merge_key(m: &RemoteMod) -> String {
    format!(
        "{}/{}",
        m.owner.to_lowercase(),
        m.name.to_lowercase()
    )
}

/// Merges BMI and Thunderstore catalogs into a single list.
///
/// On a collision (same owner + same name in both sources), the Thunderstore
/// entry wins. Distinct mods sharing a name across different owners are always
/// kept.
pub fn merge_catalogs(bmi: Vec<RemoteMod>, ts: Vec<RemoteMod>) -> Vec<RemoteMod> {
    let mut map: HashMap<String, RemoteMod> = HashMap::with_capacity(bmi.len() + ts.len());

    for m in bmi {
        map.entry(merge_key(&m)).or_insert(m);
    }
    for m in ts {
        map.insert(merge_key(&m), m);
    }

    let mut merged: Vec<RemoteMod> = map.into_values().collect();
    merged.sort_by(|a, b| a.name.cmp(&b.name));
    merged
}

pub async fn fetch_catalogs() -> Vec<RemoteMod> {
    let (bmi, ts) = tokio::join!(fetch_catalog(), fetch_ts_catalog());
    merge_catalogs(bmi, ts)
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

pub fn unzip(mut file: &File, base_path: &PathBuf, dir_name: &str) -> Result<(), String> {
    file.rewind().unwrap();
    let mut magic = [0u8; 4];
    let n = file
        .read(&mut magic)
        .map_err(|e| format!("failed to read archive: {}", e))?;
    if n != 4 || (magic != *b"PK\x03\x04" && magic != *b"PK\x05\x06" && magic != *b"PK\x07\x08") {
        return Err(String::from("file is not a zip archive"));
    }

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

    Ok(())
}

pub async fn install_mod_from_url(url: &str) -> Result<String, String> {
    let dir = dir_name_from_url(url);
    let temp_file = download_to_tmp(url).await;
    let file = temp_file.as_file();

    let mods_dir = get_balatro_appdata_dir().join("Mods");
    unzip(file, &mods_dir, &dir)?;

    Ok(format!(
        "Successfully installed mod from {} into {}",
        url,
        mods_dir.join(&dir).display()
    ))
}

fn dir_name_from_url(url: &str) -> String {
    let url_path = url.split(['?', '#']).next().unwrap_or(url);
    let file_name = url_path
        .rsplit('/')
        .find(|s| !s.is_empty())
        .unwrap_or("mod");
    let folder = file_name
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file_name);
    if folder.is_empty() {
        "mod".to_string()
    } else {
        folder.to_string()
    }
}

pub async fn install_mod_by_name(name: &str) -> Result<String, String> {
    let mut catalog = fetch_catalog().await;
    if catalog.is_empty() {
        catalog = load_catalog();
    }

    if let Some(remote) = catalog
        .iter()
        .find(|m| m.name.eq_ignore_ascii_case(name) || m.identifier.eq_ignore_ascii_case(name))
    {
        let target = install_dir(remote);
        reinstall_mod(remote, false, &target).await?;
        Ok(format!(
            "Successfully installed {} {}",
            remote.name, remote.version
        ))
    } else {
        Err(format!(
            "Could not find mod named \"{}\" in the catalog",
            name
        ))
    }
}

pub async fn reinstall_mod(remote: &RemoteMod, disabled: bool, dir: &str) -> Result<(), String> {
    let temp_file = download_to_tmp(&remote.download_url).await;
    let file = temp_file.as_file();

    let mods_dir = get_balatro_appdata_dir().join("Mods");

    unzip(file, &mods_dir, dir)?;

    if disabled {
        let ignore = mods_dir.join(dir).join(".lovelyignore");
        fs::File::create(ignore)
            .map_err(|e| format!("failed to write .lovelyignore: {}", e))?;
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mod_entry(source: &str, name: &str, owner: &str) -> RemoteMod {
        RemoteMod {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            owner: owner.to_string(),
            categories: vec![],
            repo: String::new(),
            download_url: format!("https://example.com/{}/{}", owner, name),
            folder_name: format!("{}@{}", owner, name),
            identifier: name.to_string(),
            id: name.to_string(),
            description: String::new(),
            source: source.to_string(),
            package_url: String::new(),
        }
    }

    #[test]
    fn ts_wins_on_same_owner_and_name() {
        let bmi = vec![mod_entry("bmi", "Steamodded", "notnirep")];
        let ts = vec![mod_entry("thunderstore", "Steamodded", "notnirep")];

        let merged = merge_catalogs(bmi, ts);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "thunderstore");
    }

    #[test]
    fn same_name_different_owner_both_kept() {
        let a = mod_entry("thunderstore", "cryo", "alex");
        let b = mod_entry("thunderstore", "cryo", "bob");

        let merged = merge_catalogs(vec![], vec![a, b]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn distinct_mods_kept_and_sorted() {
        let bmi_a = mod_entry("bmi", "Apple", "farmer");
        let ts_b = mod_entry("thunderstore", "Banana", "grocer");

        let merged = merge_catalogs(vec![bmi_a], vec![ts_b]);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, "Apple");
        assert_eq!(merged[1].name, "Banana");
    }

    #[test]
    fn merge_is_case_insensitive() {
        let bmi = vec![mod_entry("bmi", "Steamodded", "NotNirep")];
        let ts = vec![mod_entry("thunderstore", "Steamodded", "notnirep")];

        let merged = merge_catalogs(bmi, ts);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, "thunderstore");
    }
}
