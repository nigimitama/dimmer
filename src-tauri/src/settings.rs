//! 設定ファイル（settings.json）の読み書き

use crate::schedule::{default_schedule, normalize, ScheduleEntry};
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const CURRENT_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub version: u32,
    pub autostart: bool,
    pub schedule: Vec<ScheduleEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: CURRENT_VERSION, autostart: true, schedule: default_schedule() }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadOutcome {
    Loaded,
    Created,
    RecoveredFromCorrupt,
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s: OsString = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

pub fn backup_path(path: &Path) -> PathBuf {
    with_suffix(path, ".bak")
}

fn parse(text: &str) -> Option<Settings> {
    let settings: Settings = serde_json::from_str(text).ok()?;
    if settings.version != CURRENT_VERSION {
        return None;
    }
    let schedule = normalize(&settings.schedule).ok()?;
    Some(Settings { schedule, ..settings })
}

pub fn load_or_init(path: &Path) -> io::Result<(Settings, LoadOutcome)> {
    match fs::read_to_string(path) {
        Ok(text) => match parse(&text) {
            Some(settings) => Ok((settings, LoadOutcome::Loaded)),
            None => {
                fs::rename(path, backup_path(path))?;
                let settings = Settings::default();
                save(path, &settings)?;
                Ok((settings, LoadOutcome::RecoveredFromCorrupt))
            }
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let settings = Settings::default();
            save(path, &settings)?;
            Ok((settings, LoadOutcome::Created))
        }
        Err(e) => Err(e),
    }
}

/// 一時ファイルに書いてから置き換える。書き込み中に落ちても元のファイルは壊れない
pub fn save(path: &Path, settings: &Settings) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = with_suffix(path, ".tmp");
    let json = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

pub struct SettingsStore {
    path: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    pub fn open(path: PathBuf) -> io::Result<(Self, LoadOutcome)> {
        let (settings, outcome) = load_or_init(&path)?;
        Ok((Self { path, current: Mutex::new(settings) }, outcome))
    }

    pub fn get(&self) -> Settings {
        self.current.lock().unwrap().clone()
    }

    /// 変更を適用して保存する。保存に失敗した場合はメモリ上の値も変えない
    pub fn update(&self, f: impl FnOnce(&mut Settings)) -> io::Result<Settings> {
        let mut current = self.current.lock().unwrap();
        let mut next = current.clone();
        f(&mut next);
        save(&self.path, &next)?;
        *current = next.clone();
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::ScheduleEntry;
    use tempfile::tempdir;

    #[test]
    fn missing_file_creates_defaults() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sub").join("settings.json");
        let (settings, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::Created);
        assert_eq!(settings, Settings::default());
        assert!(settings.autostart);
        assert!(path.exists());
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let s = Settings {
            autostart: false,
            schedule: vec![ScheduleEntry { time: "12:00".into(), brightness: 40 }],
            ..Default::default()
        };
        save(&path, &s).unwrap();
        let (loaded, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(loaded, s);
        assert!(!dir.path().join("settings.json.tmp").exists());
    }

    #[test]
    fn unsorted_file_is_loaded_sorted() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{"version":1,"autostart":true,"schedule":[{"time":"18:00","brightness":80},{"time":"06:00","brightness":70}]}"#,
        )
        .unwrap();
        let (loaded, _) = load_or_init(&path).unwrap();
        assert_eq!(loaded.schedule[0].time, "06:00");
    }

    fn assert_recovers(content: &str) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, content).unwrap();
        let (settings, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::RecoveredFromCorrupt);
        assert_eq!(settings, Settings::default());
        assert_eq!(std::fs::read_to_string(backup_path(&path)).unwrap(), content);
        let (reloaded, outcome) = load_or_init(&path).unwrap();
        assert_eq!(outcome, LoadOutcome::Loaded);
        assert_eq!(reloaded, Settings::default());
    }

    #[test]
    fn broken_json_is_backed_up_and_replaced() {
        assert_recovers("{ not json");
    }

    #[test]
    fn duplicate_times_are_treated_as_corrupt() {
        assert_recovers(
            r#"{"version":1,"autostart":true,"schedule":[{"time":"06:00","brightness":70},{"time":"06:00","brightness":80}]}"#,
        );
    }

    #[test]
    fn unknown_version_is_treated_as_corrupt() {
        assert_recovers(r#"{"version":2,"autostart":true,"schedule":[]}"#);
    }

    #[test]
    fn store_update_persists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let (store, _) = SettingsStore::open(path.clone()).unwrap();
        let updated = store.update(|s| s.autostart = false).unwrap();
        assert!(!updated.autostart);
        assert!(!store.get().autostart);
        let (reloaded, _) = load_or_init(&path).unwrap();
        assert!(!reloaded.autostart);
    }
}
