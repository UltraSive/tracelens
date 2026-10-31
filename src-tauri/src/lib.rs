mod cli;
mod store;

use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use clap::Parser;
use mtr_engine::{report, PrivilegeInfo, Resolver, TraceOptions, TraceSession, TraceSnapshot};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

/// How often the UI receives fresh snapshots.
const EMIT_INTERVAL: Duration = Duration::from_millis(500);
/// Background tabs only render sparklines, so they get a trimmed sample history.
const BACKGROUND_SAMPLES: usize = 60;

#[derive(Default)]
struct Traces {
    /// Insertion-ordered so tabs keep their position across restarts.
    sessions: Mutex<Vec<TraceSession>>,
    active: Mutex<Option<u64>>,
    resolver: Resolver,
}

impl Traces {
    fn with<R>(&self, id: u64, f: impl FnOnce(&TraceSession) -> R) -> Result<R, String> {
        let map = self.sessions.lock().unwrap();
        map.iter()
            .find(|s| s.id() == id)
            .map(f)
            .ok_or_else(|| "unknown trace".into())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvInfo {
    privileges: PrivilegeInfo,
    hint: &'static str,
    os: &'static str,
    version: &'static str,
}

fn traces_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|d| d.join("traces"))
        .map_err(|e| e.to_string())
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|d| d.join("prefs.json"))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn env_info() -> EnvInfo {
    EnvInfo {
        privileges: PrivilegeInfo::discover(),
        hint: mtr_engine::permission_hint(),
        os: std::env::consts::OS,
        version: env!("CARGO_PKG_VERSION"),
    }
}

async fn start_session(resolver: Resolver, opts: TraceOptions) -> Result<TraceSession, String> {
    if opts.target.trim().is_empty() {
        return Err("Enter a hostname or IP address".into());
    }
    // DNS resolution can block; keep it off the main thread.
    tauri::async_runtime::spawn_blocking(move || TraceSession::start(opts, resolver))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_trace(
    state: State<'_, Traces>,
    opts: TraceOptions,
) -> Result<TraceSnapshot, String> {
    let session = start_session(state.resolver.clone(), opts).await?;
    let snap = session.snapshot();
    state.sessions.lock().unwrap().push(session);
    Ok(snap)
}

/// Restart a trace with the same options, keeping its tab position.
#[tauri::command]
async fn restart_trace(state: State<'_, Traces>, id: u64) -> Result<TraceSnapshot, String> {
    let opts = state.with(id, |s| s.options().clone())?;
    let session = start_session(state.resolver.clone(), opts).await?;
    let snap = session.snapshot();
    let mut list = state.sessions.lock().unwrap();
    match list.iter().position(|s| s.id() == id) {
        Some(i) => list[i] = session, // old session is dropped -> stopped
        None => list.push(session),
    }
    Ok(snap)
}

#[tauri::command]
fn set_paused(state: State<'_, Traces>, id: u64, paused: bool) -> Result<(), String> {
    state.with(id, |s| s.set_paused(paused))
}

#[tauri::command]
fn reset_trace(state: State<'_, Traces>, id: u64) -> Result<(), String> {
    state.with(id, TraceSession::clear)
}

#[tauri::command]
fn remove_trace(state: State<'_, Traces>, id: u64) {
    state.sessions.lock().unwrap().retain(|s| s.id() != id);
}

#[tauri::command]
fn set_active(state: State<'_, Traces>, id: Option<u64>) {
    *state.active.lock().unwrap() = id;
}

#[tauri::command]
fn snapshot(state: State<'_, Traces>, id: u64) -> Result<TraceSnapshot, String> {
    state.with(id, TraceSession::snapshot)
}

#[tauri::command]
fn snapshots(state: State<'_, Traces>) -> Vec<TraceSnapshot> {
    let active = *state.active.lock().unwrap();
    state
        .sessions
        .lock()
        .unwrap()
        .iter()
        .map(|s| {
            s.snapshot_with(if Some(s.id()) == active {
                usize::MAX
            } else {
                BACKGROUND_SAMPLES
            })
        })
        .collect()
}

/// Render any snapshot (live or saved) to text/markdown/csv/json.
#[tauri::command]
fn render_export(snapshot: TraceSnapshot, format: String) -> Result<String, String> {
    report::render(&snapshot, &format).ok_or_else(|| format!("unknown format '{format}'"))
}

#[tauri::command]
fn save_trace(
    app: AppHandle,
    state: State<'_, Traces>,
    id: u64,
) -> Result<store::SavedMeta, String> {
    let snap = state.with(id, TraceSession::snapshot)?;
    store::save(&traces_dir(&app)?, snap)
}

#[tauri::command]
fn list_saved(app: AppHandle) -> Result<Vec<store::SavedMeta>, String> {
    Ok(store::list(&traces_dir(&app)?))
}

#[tauri::command]
fn load_saved(app: AppHandle, file: String) -> Result<TraceSnapshot, String> {
    store::load(&traces_dir(&app)?, &file)
}

#[tauri::command]
fn delete_saved(app: AppHandle, file: String) -> Result<(), String> {
    store::delete(&traces_dir(&app)?, &file)
}

#[tauri::command]
fn get_prefs(app: AppHandle) -> Result<serde_json::Value, String> {
    Ok(store::read_prefs(&prefs_path(&app)?))
}

#[tauri::command]
fn set_prefs(app: AppHandle, prefs: serde_json::Value) -> Result<(), String> {
    store::write_prefs(&prefs_path(&app)?, &prefs)
}

fn spawn_emitter(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(EMIT_INTERVAL);
        let state = app.state::<Traces>();
        if state.sessions.lock().unwrap().is_empty() {
            continue;
        }
        let _ = app.emit("traces", snapshots(state));
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args = cli::Cli::parse();
    let initial = args.trace_options();
    if args.report {
        std::process::exit(cli::run_report(initial.expect("clap requires target")));
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Traces::default())
        .setup(move |app| {
            if let Some(opts) = initial {
                let state = app.state::<Traces>();
                match TraceSession::start(opts, state.resolver.clone()) {
                    Ok(s) => state.sessions.lock().unwrap().push(s),
                    Err(e) => eprintln!("could not start trace: {e}"),
                }
            }
            spawn_emitter(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            env_info,
            start_trace,
            restart_trace,
            set_paused,
            reset_trace,
            remove_trace,
            set_active,
            snapshot,
            snapshots,
            render_export,
            save_trace,
            list_saved,
            load_saved,
            delete_saved,
            get_prefs,
            set_prefs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
