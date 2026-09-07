//! Schema-versioned JSON persistence (spec §41).
//!
//! A single document `{ version, tasks, settings, history }` lives in the
//! Tauri app-data directory. Writes are atomic (temp file → fsync → rename)
//! and the previous-good copy is retained as `.bak` for corruption recovery.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::model::{HistoryRecord, ScheduledTask, Settings};

/// Current store schema version. Bump when the document shape changes and add
/// a migration step in [`migrate`].
pub const SCHEMA_VERSION: u32 = 1;

/// The persisted document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDocument {
    pub version: u32,
    #[serde(default)]
    pub tasks: Vec<ScheduledTask>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub history: Vec<HistoryRecord>,
}

impl Default for StoreDocument {
    fn default() -> Self {
        Self {
            version: SCHEMA_VERSION,
            tasks: Vec::new(),
            settings: Settings::default(),
            history: Vec::new(),
        }
    }
}

/// Migrate a raw document from `from_version` to [`SCHEMA_VERSION`].
///
/// Migration steps are pure functions over the raw JSON so old documents can
/// be upgraded without deserializing into the *current* model shape.
fn migrate(mut value: serde_json::Value, from_version: u32) -> AppResult<serde_json::Value> {
    let mut v = from_version;
    while v < SCHEMA_VERSION {
        match v {
            // 0 was never released; treat as equivalent to 1 (defaults apply).
            0 => {
                value
                    .as_object_mut()
                    .ok_or_else(|| {
                        AppError::PersistenceFailure("store root is not an object".into())
                    })?
                    .insert("version".into(), serde_json::json!(1));
                v = 1;
            }
            other => {
                return Err(AppError::PersistenceFailure(format!(
                    "no migration path from schema version {other}"
                )));
            }
        }
    }
    Ok(value)
}

/// JSON file store with atomic writes and corruption fallback.
pub struct JsonStore {
    path: PathBuf,
}

impl JsonStore {
    /// Create a store rooted at `path` (the main document file).
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn backup_path(&self) -> PathBuf {
        let mut p = self.path.clone().into_os_string();
        p.push(".bak");
        PathBuf::from(p)
    }

    fn tmp_path(&self) -> PathBuf {
        let mut p = self.path.clone().into_os_string();
        p.push(".tmp");
        PathBuf::from(p)
    }

    /// Load the document, applying the recovery ladder:
    /// main file → `.bak` → defaults. Returns an error only when a *newer*
    /// schema version is found (refuse to open data written by a newer app)
    /// or when migrations fail.
    pub fn load(&self) -> AppResult<StoreDocument> {
        match self.read_document(&self.path) {
            Ok(doc) => Ok(doc),
            Err(main_err) => match self.read_document(&self.backup_path()) {
                Ok(doc) => Ok(doc),
                Err(_backup_err) => {
                    // Missing main file on first run is normal — defaults.
                    if !self.path.exists() {
                        return Ok(StoreDocument::default());
                    }
                    // Present but unreadable and no valid backup: do NOT
                    // silently destroy user data; surface the failure.
                    Err(main_err)
                }
            },
        }
    }

    fn read_document(&self, path: &Path) -> AppResult<StoreDocument> {
        let raw = fs::read_to_string(path)
            .map_err(|e| AppError::PersistenceFailure(format!("read {}: {e}", path.display())))?;
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| AppError::PersistenceFailure(format!("parse {}: {e}", path.display())))?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                AppError::PersistenceFailure("store document has no version field".into())
            })?;
        if version > u64::from(SCHEMA_VERSION) {
            return Err(AppError::PersistenceFailure(format!(
                "store schema version {version} is newer than supported version {SCHEMA_VERSION}"
            )));
        }
        let migrated = migrate(value, u32::try_from(version).unwrap_or(0))?;
        serde_json::from_value(migrated)
            .map_err(|e| AppError::PersistenceFailure(format!("invalid store content: {e}")))
    }

    /// Atomically persist the document: serialize → write temp → fsync →
    /// rename over the main path, then mirror the last known-good copy to
    /// `.bak` for corruption recovery.
    pub fn save(&self, doc: &StoreDocument) -> AppResult<()> {
        let mut doc = doc.clone();
        doc.version = SCHEMA_VERSION;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::PersistenceFailure(format!("create dir {}: {e}", parent.display()))
            })?;
        }

        let serialized = serde_json::to_string_pretty(&doc)
            .map_err(|e| AppError::PersistenceFailure(format!("serialize: {e}")))?;

        let tmp = self.tmp_path();
        {
            // Scope closes the file handle before rename (Windows requires it).
            let mut f = fs::File::create(&tmp)
                .map_err(|e| AppError::PersistenceFailure(format!("create tmp: {e}")))?;
            f.write_all(serialized.as_bytes())
                .and_then(|()| f.sync_all())
                .map_err(|e| AppError::PersistenceFailure(format!("write tmp: {e}")))?;
        }
        fs::rename(&tmp, &self.path)
            .map_err(|e| AppError::PersistenceFailure(format!("rename: {e}")))?;
        // Keep a mirror of the last known-good document for corruption
        // recovery. A failure here is non-fatal (main file is already good).
        let _ = fs::copy(&self.path, self.backup_path());
        Ok(())
    }

    /// Load, apply `f`, and persist — a single transactional helper.
    pub fn update<T>(&self, f: impl FnOnce(&mut StoreDocument) -> AppResult<T>) -> AppResult<T> {
        let mut doc = self.load()?;
        let result = f(&mut doc)?;
        self.save(&doc)?;
        Ok(result)
    }
}

/// Enforce the history bound (default max 1000, oldest evicted).
pub fn bound_history(history: &mut Vec<HistoryRecord>, limit: u32) {
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    if history.len() > limit {
        // Records are appended chronologically; drop the oldest.
        let excess = history.len() - limit;
        history.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, ExecutionOutcome, Key, MisfirePolicy, Schedule};
    use chrono::Utc;
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("agent-pulse-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("test tmpdir");
        dir
    }

    fn sample_task(name: &str) -> ScheduledTask {
        ScheduledTask::new(
            name,
            Default::default(),
            Schedule::After {
                hours: 1,
                minutes: 0,
                seconds: 0,
            },
            vec![
                Action::FocusTarget,
                Action::TypeText {
                    text: "continue".into(),
                },
                Action::PressKey {
                    key: Key::Enter,
                    count: 1,
                    interval_ms: 0,
                },
            ],
            MisfirePolicy::RunImmediately,
            Utc::now(),
        )
        .expect("test task")
    }

    fn sample_record() -> HistoryRecord {
        let now = Utc::now();
        HistoryRecord {
            id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            task_name: "t".into(),
            scheduled_at: now,
            started_at: now,
            finished_at: now,
            target_description: "w".into(),
            outcome: ExecutionOutcome::Success,
        }
    }

    #[test]
    fn round_trip_tasks_settings_history() {
        let dir = temp_dir();
        let store = JsonStore::new(dir.join("store.json"));
        let mut doc = StoreDocument::default();
        doc.tasks.push(sample_task("a"));
        doc.tasks.push(sample_task("b"));
        doc.history.push(sample_record());
        doc.settings.theme = crate::model::Theme::Dark;
        store.save(&doc).expect("test save");

        let loaded = store.load().expect("test load");
        assert_eq!(loaded.tasks.len(), 2);
        assert_eq!(loaded.tasks[0].name, "a");
        assert_eq!(loaded.history.len(), 1);
        assert_eq!(loaded.settings.theme, crate::model::Theme::Dark);
        assert_eq!(loaded.version, SCHEMA_VERSION);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corruption_falls_back_to_backup() {
        let dir = temp_dir();
        let path = dir.join("store.json");
        let store = JsonStore::new(path.clone());

        let mut doc = StoreDocument::default();
        doc.tasks.push(sample_task("keep-me"));
        store.save(&doc).expect("test save");

        // Corrupt the main file; .bak still holds the good copy.
        fs::write(&path, "{not json at all").expect("test corrupt");

        let loaded = store.load().expect("test load from backup");
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].name, "keep-me");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = temp_dir();
        let store = JsonStore::new(dir.join("nonexistent.json"));
        let doc = store.load().expect("test load");
        assert!(doc.tasks.is_empty());
        assert_eq!(doc.settings, Settings::default());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn both_corrupt_surfaces_persistence_failure() {
        let dir = temp_dir();
        let path = dir.join("store.json");
        fs::write(&path, "garbage{{{{").expect("test write");
        fs::write(dir.join("store.json.bak"), "also garbage]]]]").expect("test write");
        let store = JsonStore::new(path);
        let result = store.load();
        assert!(matches!(result, Err(AppError::PersistenceFailure(_))));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn newer_schema_version_refuses_load() {
        let dir = temp_dir();
        let path = dir.join("store.json");
        fs::write(
            &path,
            r#"{"version": 99, "tasks": [], "settings": {}, "history": []}"#,
        )
        .expect("test write");
        let store = JsonStore::new(path);
        assert!(matches!(store.load(), Err(AppError::PersistenceFailure(_))));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn older_schema_version_migrates() {
        let dir = temp_dir();
        let path = dir.join("store.json");
        // version 0: no settings field at all; migration fills defaults.
        fs::write(&path, r#"{"version": 0, "tasks": [], "history": []}"#).expect("test write");
        let store = JsonStore::new(path);
        let doc = store.load().expect("test load");
        assert_eq!(doc.settings, Settings::default());
        assert_eq!(doc.version, SCHEMA_VERSION);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_is_atomic_no_tmp_left_behind() {
        let dir = temp_dir();
        let store = JsonStore::new(dir.join("store.json"));
        store.save(&StoreDocument::default()).expect("test save");
        assert!(!dir.join("store.json.tmp").exists());
        assert!(dir.join("store.json").exists());
        assert!(dir.join("store.json.bak").exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn update_applies_and_persists() {
        let dir = temp_dir();
        let store = JsonStore::new(dir.join("store.json"));
        store
            .update(|doc| {
                doc.tasks.push(sample_task("via-update"));
                Ok(())
            })
            .expect("test update");
        let loaded = store.load().expect("test load");
        assert_eq!(loaded.tasks.len(), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn history_bound_evicts_oldest() {
        let mut history: Vec<HistoryRecord> = (0..1200).map(|_| sample_record()).collect();
        bound_history(&mut history, 1000);
        assert_eq!(history.len(), 1000);
        bound_history(&mut history, 1000);
        assert_eq!(history.len(), 1000);
    }
}
