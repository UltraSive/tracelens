//! On-disk persistence: preferences (opaque JSON owned by the UI) and saved traces.
use std::fs;
use std::path::{Path, PathBuf};

use mtr_engine::TraceSnapshot;
use serde::{Deserialize, Serialize};

const SAVED_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SavedFile {
    version: u32,
    saved_at: i64,
    snapshot: TraceSnapshot,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedMeta {
    pub file: String,
    pub target: String,
    pub target_addr: String,
    pub protocol: mtr_engine::Protocol,
    pub saved_at: i64,
    pub started_at: i64,
    pub rounds: usize,
    pub hops: usize,
    pub headline: String,
    pub level: mtr_engine::VerdictLevel,
}

fn meta(file: String, f: &SavedFile) -> SavedMeta {
    let s = &f.snapshot;
    SavedMeta {
        file,
        target: s.target.clone(),
        target_addr: s.target_addr.to_string(),
        protocol: s.options.protocol,
        saved_at: f.saved_at,
        started_at: s.started_at,
        rounds: s.rounds,
        hops: s.hops.len(),
        headline: s.verdict.headline.clone(),
        level: s.verdict.level,
    }
}

/// Only plain file names we generated are accepted (no path traversal).
fn checked(dir: &Path, file: &str) -> Result<PathBuf, String> {
    let ok = file.ends_with(".json")
        && !file.starts_with('.')
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if ok {
        Ok(dir.join(file))
    } else {
        Err("invalid file name".into())
    }
}

pub fn save(dir: &Path, snapshot: TraceSnapshot) -> Result<SavedMeta, String> {
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let now = chrono_now();
    let slug: String = snapshot
        .target
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect();
    let file = format!("{now}-{slug}.json");
    let data = SavedFile {
        version: SAVED_VERSION,
        saved_at: now,
        snapshot,
    };
    let json = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    fs::write(checked(dir, &file)?, json).map_err(|e| e.to_string())?;
    Ok(meta(file, &data))
}

pub fn list(dir: &Path) -> Vec<SavedMeta> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    let mut out: Vec<SavedMeta> = entries
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().into_string().ok()?;
            let path = checked(dir, &file).ok()?;
            let f: SavedFile = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
            Some(meta(file, &f))
        })
        .collect();
    out.sort_by_key(|m| std::cmp::Reverse(m.saved_at));
    out
}

pub fn load(dir: &Path, file: &str) -> Result<TraceSnapshot, String> {
    let bytes = fs::read(checked(dir, file)?).map_err(|e| e.to_string())?;
    let f: SavedFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("unreadable trace file: {e}"))?;
    Ok(f.snapshot)
}

pub fn delete(dir: &Path, file: &str) -> Result<(), String> {
    fs::remove_file(checked(dir, file)?).map_err(|e| e.to_string())
}

pub fn read_prefs(path: &Path) -> serde_json::Value {
    fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

pub fn write_prefs(path: &Path, prefs: &serde_json::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(prefs).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
