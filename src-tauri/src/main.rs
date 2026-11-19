#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod state;

use multipass_core::{InventoryEntry, NcFile, ToolMap};
use serde::Serialize;
use state::{sort_paths, AppState, CommandError, FileInfo, LoadError, Prepared, Saved};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadProgress {
    done: usize,
    total: usize,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadOutcome {
    files: Vec<FileInfo>,
    errors: Vec<LoadError>,
}

#[tauri::command]
async fn load_files<R: Runtime>(app: AppHandle<R>, paths: Vec<String>) -> Result<LoadOutcome, CommandError> {
    let mut paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).filter(|p| !p.is_dir()).collect();
    sort_paths(&mut paths);
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let st = handle.state::<AppState>();
        let total = paths.len();
        let mut files = Vec::new();
        let mut errors = Vec::new();
        for (i, p) in paths.into_iter().enumerate() {
            let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let _ = handle.emit("load-progress", LoadProgress { done: i, total, name: name.clone() });
            match NcFile::read(&p) {
                Ok(f) => files.push(st.insert(p, f)),
                Err(e) => errors.push(LoadError {
                    name,
                    path: p.to_string_lossy().into_owned(),
                    message: e.to_string(),
                    error: e,
                }),
            }
        }
        let _ = handle.emit("load-progress", LoadProgress { done: total, total, name: String::new() });
        LoadOutcome { files, errors }
    })
    .await
    .map_err(|e| CommandError::plain(e.to_string()))
}

#[tauri::command]
fn forget_files(state: State<'_, AppState>, ids: Vec<u64>) {
    state.forget(&ids);
}

#[tauri::command]
fn auto_tools(
    state: State<'_, AppState>,
    ids: Vec<u64>,
    inventory: Vec<InventoryEntry>,
) -> Result<Vec<ToolMap>, CommandError> {
    state.auto_tools(&ids, &inventory)
}

#[tauri::command]
async fn prepare_merge<R: Runtime>(
    app: AppHandle<R>,
    ids: Vec<u64>,
    maps: Vec<ToolMap>,
    restore: bool,
    safe_z: f64,
    stamp: String,
) -> Result<Prepared, CommandError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().prepare(&ids, &maps, restore, safe_z, stamp))
        .await
        .map_err(|e| CommandError::plain(e.to_string()))?
}

#[tauri::command]
async fn save_merge<R: Runtime>(app: AppHandle<R>, path: String) -> Result<Saved, CommandError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().save(&PathBuf::from(path)))
        .await
        .map_err(|e| CommandError::plain(e.to_string()))?
}

#[tauri::command]
fn initial_files() -> Vec<String> {
    std::env::args().skip(1).filter(|a| PathBuf::from(a).is_file()).collect()
}

fn setup<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.manage(AppState::default()).invoke_handler(tauri::generate_handler![
        load_files,
        forget_files,
        auto_tools,
        prepare_merge,
        save_merge,
        initial_files
    ])
}

fn main() {
    setup(tauri::Builder::default().plugin(tauri_plugin_dialog::init()))
        .run(tauri::generate_context!())
        .expect("MultiPass konnte nicht gestartet werden");
}

#[cfg(test)]
mod ipc_tests;
