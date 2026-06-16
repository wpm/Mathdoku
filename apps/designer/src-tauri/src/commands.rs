#![allow(
    clippy::needless_pass_by_value, // Tauri commands must take args by value
    clippy::must_use_candidate,     // Tauri handles return values via IPC
)]

use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string};

use mathdoku::{CageOperator, Cell, Polyomino, Target};
use mathdoku_designer_core::{self as core, AppState, DocState, SaveResult, State};
#[cfg(feature = "without-solution")]
use tauri::menu::MenuItem;
use tauri::{AppHandle, Emitter, Manager, Runtime, State as TauriState};

/// Handles to the native Puzzle menu's Fix / Unfix items, stored in app state so
/// [`set_puzzle_menu_enabled`] can toggle their enabled state from the frontend.
#[cfg(feature = "without-solution")]
pub struct PuzzleMenu<R: Runtime> {
    pub fix: MenuItem<R>,
    pub unfix: MenuItem<R>,
}

/// Filename of the recent-file record stored in the app data directory.
pub const RECENT_FILE: &str = "last_open.json";

// ---- recent-file helpers ----

/// Returns the path of the recent-file record (`last_open.json`) in the app data directory.
pub fn recent_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join(RECENT_FILE))
}

/// Writes or removes the recent-file record.
///
/// `path = Some(p)` writes `{ path: p, active: … }` to `last_open.json`.
/// `path = None` deletes `last_open.json` so the next launch starts fresh.
pub fn write_recent<R: Runtime>(app: &AppHandle<R>, path: Option<&str>, active: Option<Cell>) {
    #[derive(Serialize)]
    struct Record<'a> {
        path: Option<&'a str>,
        active: Option<Cell>,
    }
    let Some(file) = recent_path(app) else { return };
    match path {
        Some(p) => {
            if let Some(parent) = file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = to_string(&Record {
                path: Some(p),
                active,
            }) {
                let _ = fs::write(file, json);
            }
        }
        None => {
            let _ = fs::remove_file(file);
        }
    }
}

/// Deserialized contents of `last_open.json`.
#[derive(Deserialize)]
pub struct RecentRecord {
    pub path: Option<String>,
    #[serde(default)]
    pub active: Option<Cell>,
}

/// Reads and parses `last_open.json`, returning `None` if the file is absent or malformed.
pub fn read_recent<R: Runtime>(app: &AppHandle<R>) -> Option<RecentRecord> {
    let file = recent_path(app)?;
    let content = fs::read_to_string(file).ok()?;
    from_str::<RecentRecord>(&content).ok()
}

// ---- commands ----

/// Creates a new empty *n*×*n* Without-Solution puzzle.
///
/// # Errors
/// Returns an error string if `n` is invalid or the state lock is poisoned.
#[cfg(feature = "without-solution")]
#[tauri::command]
pub fn new_empty(n: usize, state: TauriState<Mutex<AppState>>) -> Result<State, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    core::new_empty(&mut s, n).map_err(|e| e.to_string())
}

/// Creates a new puzzle whose solution is a random Latin square.
///
/// # Errors
/// Returns an error string if `n` is invalid or the state lock is poisoned.
#[tauri::command]
pub fn new_latin_square(n: usize, state: TauriState<Mutex<AppState>>) -> Result<State, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    core::new_latin_square(&mut s, n, &mut rand::rng()).map_err(|e| e.to_string())
}

/// # Errors
/// Returns an error string if no puzzle is loaded, serialization fails, or the file cannot be
/// written.
#[tauri::command]
pub fn save_puzzle<R: Runtime>(
    path: String,
    app: AppHandle<R>,
    state: TauriState<Mutex<AppState>>,
) -> Result<SaveResult, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let json = core::serialize_save(&s).map_err(|e| e.to_string())?;
    fs::write(&path, &json).map_err(|e| e.to_string())?;
    s.path = Some(path.clone());
    s.dirty = false;
    let active = s.active;
    drop(s);
    write_recent(&app, Some(&path), active);
    Ok(SaveResult { path })
}

/// # Errors
/// Returns an error string if the file cannot be read, JSON is malformed, or the version is
/// unsupported.
#[tauri::command]
pub fn load_puzzle<R: Runtime>(
    path: String,
    app: AppHandle<R>,
    state: TauriState<Mutex<AppState>>,
) -> Result<State, String> {
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut s = state.lock().map_err(|e| e.to_string())?;
    let designer_state = core::apply_loaded(&mut s, &json).map_err(|e| e.to_string())?;
    s.path = Some(path.clone());
    drop(s);
    write_recent(&app, Some(&path), None);
    Ok(designer_state)
}

/// Returns the document state (dirty flag and current file path).
#[tauri::command]
pub fn get_doc_state(state: TauriState<Mutex<AppState>>) -> DocState {
    let s = state.lock().unwrap_or_else(PoisonError::into_inner);
    core::get_doc_state(&s)
}

/// Returns the current designer [`State`], or `None` if no puzzle is loaded.
///
/// Called at startup so the frontend can restore the last session.
#[tauri::command]
pub fn get_puzzle(state: TauriState<Mutex<AppState>>) -> Option<State> {
    let s = state.lock().ok()?;
    core::get_puzzle(&s)
}

/// Persists the active cell position.
///
/// # Errors
/// Returns an error string if the state lock is poisoned.
#[tauri::command]
pub fn set_active_cell<R: Runtime>(
    active: Cell,
    app: AppHandle<R>,
    state: TauriState<Mutex<AppState>>,
) -> Result<(), String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    core::set_active_cell(&mut s, active);
    let path = s.path.clone();
    drop(s);
    write_recent(&app, path.as_deref(), Some(active));
    Ok(())
}

/// Exits the application immediately.
#[tauri::command]
pub fn quit_app<R: Runtime>(app: AppHandle<R>) {
    app.exit(0);
}

/// # Errors
/// Returns an error string if no window is found or the title cannot be set.
#[tauri::command]
pub fn set_window_title<R: Runtime>(title: String, app: AppHandle<R>) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?
        .set_title(&title)
        .map_err(|e| e.to_string())
}

/// Adds a cage to the current puzzle for the given cells and operator.
///
/// # Errors
/// Returns an error string if no puzzle is loaded or `operator` is not valid
/// for the polyomino size.
#[tauri::command]
pub fn insert_cage(
    polyomino: Polyomino,
    operator: CageOperator,
    target: Option<Target>,
    state: TauriState<Mutex<AppState>>,
) -> Result<State, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    core::insert_cage(&mut s, polyomino, operator, target).map_err(|e| e.to_string())
}

/// Switches the current puzzle from Without-Solution to With-Solution by
/// snapshotting its unique completion into `solution`.
///
/// # Errors
/// Returns an error string if no puzzle is loaded or the puzzle does not have
/// exactly one global completion.
#[cfg(feature = "without-solution")]
#[tauri::command]
pub fn fix(state: TauriState<Mutex<AppState>>) -> Result<State, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    core::fix(&mut s).map_err(|e| e.to_string())
}

/// Switches the current puzzle from With-Solution to Without-Solution by
/// discarding its `solution`.
///
/// # Errors
/// Returns an error string if no puzzle is loaded.
#[cfg(feature = "without-solution")]
#[tauri::command]
pub fn unfix(state: TauriState<Mutex<AppState>>) -> Result<State, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    core::unfix(&mut s).map_err(|e| e.to_string())
}

/// Updates the enabled state of the native Puzzle menu's Fix / Unfix items.
///
/// The frontend pushes this whenever the puzzle's mode or unique-solution status
/// changes so the menu reflects which transition is currently valid.
///
/// # Errors
/// Returns an error string if setting either item's enabled state fails.
#[cfg(feature = "without-solution")]
#[tauri::command]
pub fn set_puzzle_menu_enabled<R: Runtime>(
    fix_enabled: bool,
    unfix_enabled: bool,
    app: AppHandle<R>,
) -> Result<(), String> {
    // `AppHandle<R>` ties the command's runtime so the `PuzzleMenu<R>` lookup
    // resolves; a bare `State<PuzzleMenu<R>>` arg leaves `R` unconstrained.
    let menu = app.state::<PuzzleMenu<R>>();
    menu.fix
        .set_enabled(fix_enabled)
        .map_err(|e| e.to_string())?;
    menu.unfix
        .set_enabled(unfix_enabled)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Removes the cage covering `polyomino` from the current puzzle.
///
/// # Errors
/// Returns an error string if no puzzle is loaded or no cage covers exactly `polyomino`.
#[tauri::command]
pub fn remove_cage_at(
    polyomino: Polyomino,
    state: TauriState<Mutex<AppState>>,
) -> Result<State, String> {
    let mut s = state.lock().map_err(|e| e.to_string())?;
    core::remove_cage_at(&mut s, &polyomino).map_err(|e| e.to_string())
}

// ---- auto-update ----
//
// Tauri's updater never runs on its own: registering the plugin only makes the
// capability available, so the app has to call `check()` / `download_and_install()`
// itself. These two commands are the backend half of that flow; the frontend
// drives them from its mount (see `app.rs`). The updater plugin is desktop-only,
// so the bodies are gated on `desktop`; the mobile stubs keep the commands in
// the IPC handler (which can't carry `#[cfg]` per entry) and just report that
// updating is unsupported there.

/// The Tauri event carrying download progress.
///
/// Payload is `{ downloaded, total }`, where `downloaded` is the running byte
/// count and `total` is the content length (or `null` when the server doesn't
/// announce one — an indeterminate bar).
pub const EVENT_UPDATE_PROGRESS: &str = "update://progress";
/// The Tauri event signalling the download has finished and install is starting.
pub const EVENT_UPDATE_DOWNLOAD_FINISHED: &str = "update://download-finished";

/// Outcome of an update check: whether a newer release is available and, if so,
/// its version string (shown in the Updating modal).
#[derive(Serialize)]
pub struct UpdateCheck {
    pub available: bool,
    pub version: Option<String>,
}

/// Running download progress emitted over [`EVENT_UPDATE_PROGRESS`].
#[cfg(desktop)]
#[derive(Clone, Serialize)]
struct UpdateProgress {
    downloaded: usize,
    total: Option<u64>,
}

/// Checks the configured endpoint for a newer release without downloading it.
///
/// # Errors
/// Returns an error string if the updater is unavailable (e.g. the
/// `updater:default` permission is missing) or the check request fails.
#[cfg(desktop)]
#[tauri::command]
pub async fn check_for_update<R: Runtime>(app: AppHandle<R>) -> Result<UpdateCheck, String> {
    use tauri_plugin_updater::UpdaterExt;
    match app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
    {
        Some(update) => Ok(UpdateCheck {
            available: true,
            version: Some(update.version),
        }),
        None => Ok(UpdateCheck {
            available: false,
            version: None,
        }),
    }
}

/// Desktop-only command; the mobile stub reports that updating is unsupported.
#[cfg(not(desktop))]
#[tauri::command]
pub async fn check_for_update<R: Runtime>(app: AppHandle<R>) -> Result<UpdateCheck, String> {
    let _ = &app;
    Err("Updating is not supported on this platform.".to_string())
}

/// Downloads and installs the available update, then relaunches.
///
/// Re-checks for an update, then downloads and installs it, emitting
/// [`EVENT_UPDATE_PROGRESS`] as bytes arrive and [`EVENT_UPDATE_DOWNLOAD_FINISHED`]
/// once the download completes, and finally relaunches into the new version.
///
/// The function re-`check()`s rather than trusting a prior result so the handle
/// to the update is fresh; the `total` byte count may be `None`, which the
/// frontend renders as an indeterminate bar.
///
/// # Errors
/// Returns an error string if the updater is unavailable, no update is found, or
/// the download/install fails. On success it does not return — the app restarts.
#[cfg(desktop)]
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update is available to install.".to_string())?;

    let mut downloaded: usize = 0;
    update
        .download_and_install(
            |chunk, total| {
                downloaded = downloaded.saturating_add(chunk);
                let _ = app.emit(EVENT_UPDATE_PROGRESS, UpdateProgress { downloaded, total });
            },
            || {
                let _ = app.emit(EVENT_UPDATE_DOWNLOAD_FINISHED, ());
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    // Relaunch into the freshly installed version. `restart` never returns.
    app.restart()
}

/// Desktop-only command; the mobile stub reports that updating is unsupported.
#[cfg(not(desktop))]
#[tauri::command]
pub async fn install_update<R: Runtime>(app: AppHandle<R>) -> Result<(), String> {
    let _ = &app;
    Err("Updating is not supported on this platform.".to_string())
}
