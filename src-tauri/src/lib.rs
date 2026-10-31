use std::collections::BTreeMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use mtr_engine::{report, PrivilegeInfo, Resolver, TraceOptions, TraceSession, TraceSnapshot};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

/// How often the UI receives fresh snapshots.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Default)]
struct Traces {
    sessions: Mutex<BTreeMap<u64, TraceSession>>,
    resolver: Resolver,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvInfo {
    privileges: PrivilegeInfo,
    hint: &'static str,
    os: &'static str,
}

#[tauri::command]
fn env_info() -> EnvInfo {
    EnvInfo {
        privileges: PrivilegeInfo::discover(),
        hint: mtr_engine::permission_hint(),
        os: std::env::consts::OS,
    }
}

#[tauri::command]
async fn start_trace(state: State<'_, Traces>, opts: TraceOptions) -> Result<TraceSnapshot, String> {
    if opts.target.trim().is_empty() {
        return Err("Enter a hostname or IP address".into());
    }
    // DNS resolution can block; keep it off the main thread.
    let resolver = state.resolver.clone();
    let session = tauri::async_runtime::spawn_blocking(move || TraceSession::start(opts, resolver))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let snap = session.snapshot();
    state.sessions.lock().unwrap().insert(session.id(), session);
    Ok(snap)
}

#[tauri::command]
fn stop_trace(state: State<'_, Traces>, id: u64) {
    if let Some(s) = state.sessions.lock().unwrap().get(&id) {
        s.stop();
    }
}

/// Restart a trace with the same options, replacing the old one in place.
#[tauri::command]
async fn restart_trace(state: State<'_, Traces>, id: u64) -> Result<TraceSnapshot, String> {
    let opts = {
        let mut map = state.sessions.lock().unwrap();
        let old = map.remove(&id).ok_or("unknown trace")?;
        old.stop();
        old.options().clone()
    };
    start_trace(state, opts).await
}

#[tauri::command]
fn reset_trace(state: State<'_, Traces>, id: u64) {
    if let Some(s) = state.sessions.lock().unwrap().get(&id) {
        s.clear();
    }
}

#[tauri::command]
fn remove_trace(state: State<'_, Traces>, id: u64) {
    if let Some(s) = state.sessions.lock().unwrap().remove(&id) {
        s.stop();
    }
}

#[tauri::command]
fn export_trace(state: State<'_, Traces>, id: u64, format: String) -> Result<String, String> {
    let map = state.sessions.lock().unwrap();
    let snap = map.get(&id).ok_or("unknown trace")?.snapshot();
    Ok(match format.as_str() {
        "csv" => report::to_csv(&snap),
        "json" => report::to_json(&snap),
        _ => report::to_text(&snap),
    })
}

#[tauri::command]
fn snapshots(state: State<'_, Traces>) -> Vec<TraceSnapshot> {
    state.sessions.lock().unwrap().values().map(TraceSession::snapshot).collect()
}

fn spawn_emitter(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(EMIT_INTERVAL);
        let state = app.state::<Traces>();
        let snaps: Vec<TraceSnapshot> = {
            let map = state.sessions.lock().unwrap();
            if map.is_empty() {
                continue;
            }
            map.values().map(TraceSession::snapshot).collect()
        };
        let _ = app.emit("traces", snaps);
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Traces::default())
        .setup(|app| {
            spawn_emitter(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            env_info,
            start_trace,
            stop_trace,
            restart_trace,
            reset_trace,
            remove_trace,
            export_trace,
            snapshots
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
