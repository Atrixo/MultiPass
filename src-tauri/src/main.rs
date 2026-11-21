#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod state;

use multipass_core::{InventoryEntry, NcError, NcFile, ToolMap};
use serde::Serialize;
use state::{sort_paths, AppState, CommandError, EstimateOutcome, FileInfo, FilePreview, LoadError, Prepared, Saved};
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
            match NcFile::read(&p).and_then(|f| {
                if f.is_merged() {
                    Err(NcError::AlreadyMerged { file: f.name })
                } else {
                    Ok(f)
                }
            }) {
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
async fn estimate<R: Runtime>(app: AppHandle<R>, ids: Vec<u64>, safe_z: f64) -> Result<EstimateOutcome, CommandError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().estimate(&ids, safe_z))
        .await
        .map_err(|e| CommandError::plain(e.to_string()))?
}

#[tauri::command]
async fn preview<R: Runtime>(
    app: AppHandle<R>,
    ids: Vec<u64>,
    restore: bool,
    safe_z: f64,
) -> Result<Vec<FilePreview>, CommandError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().preview(&ids, restore, safe_z))
        .await
        .map_err(|e| CommandError::plain(e.to_string()))?
}

#[tauri::command]
async fn timeline<R: Runtime>(
    app: AppHandle<R>,
    ids: Vec<u64>,
    restore: bool,
    safe_z: f64,
) -> Result<tauri::ipc::Response, CommandError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<AppState>().timeline(&ids, restore, safe_z))
        .await
        .map_err(|e| CommandError::plain(e.to_string()))?
        .map(tauri::ipc::Response::new)
}

#[tauri::command]
fn default_inventory() -> Vec<InventoryEntry> {
    multipass_core::default_inventory()
}

#[tauri::command]
fn initial_files() -> Vec<String> {
    file_args(std::env::args().skip(1), &std::env::current_dir().unwrap_or_default())
}

fn file_args<I: IntoIterator<Item = String>>(args: I, cwd: &std::path::Path) -> Vec<String> {
    args.into_iter()
        .filter(|a| !a.starts_with('-'))
        .map(|a| {
            let p = PathBuf::from(&a);
            if p.is_absolute() {
                p
            } else {
                cwd.join(p)
            }
        })
        .filter(|p| p.is_file())
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

fn open_in_window<R: Runtime>(app: &AppHandle<R>, files: Vec<String>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
    if !files.is_empty() {
        let _ = app.emit("open-files", files);
    }
}

fn setup<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.manage(AppState::default()).invoke_handler(tauri::generate_handler![
        load_files,
        forget_files,
        auto_tools,
        prepare_merge,
        save_merge,
        initial_files,
        default_inventory,
        estimate,
        preview,
        timeline
    ])
}

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            open_in_window(app, file_args(argv.into_iter().skip(1), std::path::Path::new(&cwd)));
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init());
    let app = setup(builder).build(tauri::generate_context!()).expect("MultiPass konnte nicht gestartet werden");
    app.run(|_handle, _event| {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let tauri::RunEvent::Opened { urls } = _event {
            let files = urls
                .into_iter()
                .filter_map(|u| u.to_file_path().ok())
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            open_in_window(_handle, files);
        }
    });
}

#[cfg(test)]
mod ipc_tests;

#[cfg(test)]
mod tests {
    use super::file_args;

    #[test]
    fn dateien_aus_aufrufargumenten() {
        let dir = std::env::temp_dir().join(format!("multipass_args_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.cnc"), "x").unwrap();
        let abs = dir.join("a.cnc").to_string_lossy().into_owned();
        let got = file_args(vec!["a.cnc".into(), "--flag".into(), abs.clone(), "fehlt.cnc".into()], &dir);
        assert_eq!(got, vec![abs.clone(), abs]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
