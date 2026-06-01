//! Centralized IPC surface for the designer frontend.
//!
//! The frontend calls the typed wrappers in this module instead of passing
//! stringly-typed command names and `JsValue` blobs around, so a renamed
//! command or a mismatched argument shape becomes a compile error rather than a
//! runtime failure. Reading this file gives the full IPC contract in one place.
//!
//! Each wrapper keeps one public signature but has two implementations selected
//! at compile time (ADR-0002):
//!
//! * The **default (Tauri)** build serializes the arguments, awaits Tauri's
//!   `invoke`, and deserializes the response.
//! * The **`web`** build calls the matching `mathdoku-designer-core` function
//!   directly against the `thread_local!<RefCell<AppState>>` singleton in
//!   [`crate::web_shims`] — no serialization round-trip, no `JsValue` dance, no
//!   `window.__TAURI__`.
//!
//! The rest of the frontend is oblivious to which backend is compiled in.

#![allow(
    clippy::future_not_send,         // WASM async is inherently single-threaded
    clippy::missing_errors_doc,      // every wrapper's error is "the backend call failed"
    clippy::unused_async,            // web wrappers keep the async signature without awaiting
    unused_results,                  // quit_app discards its fire-and-forget JsValue
)]

use mathdoku::{Cell, Operator, Polyomino, Target};
use mathdoku_designer_core::{DocState, SaveResult, State};

// Default-build (Tauri IPC) imports.
#[cfg(not(feature = "web"))]
use serde::Serialize;
#[cfg(not(feature = "web"))]
use serde::de::DeserializeOwned;
#[cfg(not(feature = "web"))]
use serde_wasm_bindgen::{from_value, to_value};
#[cfg(not(feature = "web"))]
use wasm_bindgen::prelude::*;

// Web-build (direct core call) imports.
#[cfg(feature = "web")]
use crate::web_shims::APP_STATE;
#[cfg(feature = "web")]
use mathdoku_designer_core as core;

// ---- Tauri bindings (default build only) ----
//
// These vanish entirely from the web build, which never touches
// `window.__TAURI__`.
#[cfg(not(feature = "web"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke")]
    async fn raw_invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"], js_name = "open")]
    async fn dialog_open(options: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"], js_name = "save")]
    async fn dialog_save(options: JsValue) -> JsValue;
}

/// An error crossing the IPC boundary.
#[derive(Debug, Clone)]
pub enum IpcError {
    /// The backend ran but returned `Err(String)`.
    Command(String),
    /// Serializing the arguments or deserializing the response failed.
    Serde(String),
}

impl std::fmt::Display for IpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(msg) | Self::Serde(msg) => f.write_str(msg),
        }
    }
}

// ---- argument shapes (default build only) ----

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct NewPuzzleArgs {
    n: usize,
}

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct PathArgs {
    path: String,
}

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct ActiveArgs {
    active: Cell,
}

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct InsertCageArgs {
    cells: Vec<Cell>,
    operator: Operator,
    /// `Some` in Without-Solution mode (author-chosen target); `None` in
    /// With-Solution mode (the backend derives the target from the solution).
    target: Option<Target>,
}

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct RemoveCageAtArgs {
    polyomino: Polyomino,
}

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct TitleArgs {
    title: String,
}

// ---- low-level call helpers (default build only) ----

/// Detects the `Err(String)` arm of a Tauri command result.
///
/// Tauri serializes `Err(String)` as a plain JS string and `Ok(T)` as `T`'s
/// JSON. No command returns a bare string on success, so a string value here
/// unambiguously means the command failed.
#[cfg(not(feature = "web"))]
fn command_error(value: &JsValue) -> Option<IpcError> {
    value.as_string().map(IpcError::Command)
}

/// Invokes a command whose Rust signature returns `Result<R, String>` and
/// deserializes the success payload into `R`.
#[cfg(not(feature = "web"))]
async fn call<A, R>(cmd: &str, args: A) -> Result<R, IpcError>
where
    A: Serialize,
    R: DeserializeOwned,
{
    let args = to_value(&args).map_err(|e| IpcError::Serde(e.to_string()))?;
    let result = raw_invoke(cmd, args).await;
    if let Some(err) = command_error(&result) {
        return Err(err);
    }
    from_value(result).map_err(|e| IpcError::Serde(e.to_string()))
}

/// Invokes a command whose Rust signature returns `Result<(), String>`,
/// surfacing any command error but discarding the (null) success payload.
#[cfg(not(feature = "web"))]
async fn call_unit<A: Serialize>(cmd: &str, args: A) -> Result<(), IpcError> {
    let args = to_value(&args).map_err(|e| IpcError::Serde(e.to_string()))?;
    let result = raw_invoke(cmd, args).await;
    command_error(&result).map_or(Ok(()), Err)
}

/// Invokes a no-argument command returning `Result<R, String>`.
#[cfg(not(feature = "web"))]
async fn call_no_args<R: DeserializeOwned>(cmd: &str) -> Result<R, IpcError> {
    let result = raw_invoke(cmd, JsValue::NULL).await;
    if let Some(err) = command_error(&result) {
        return Err(err);
    }
    from_value(result).map_err(|e| IpcError::Serde(e.to_string()))
}

// ---- web call helper ----

/// Runs `f` against the thread-local `AppState`, mapping a core [`core::Error`]
/// onto [`IpcError::Command`] so the web path reports the same error shape the
/// Tauri path does.
#[cfg(feature = "web")]
fn with_state<R>(
    f: impl FnOnce(&mut core::AppState) -> Result<R, core::Error>,
) -> Result<R, IpcError> {
    APP_STATE
        .with(|s| f(&mut s.borrow_mut()))
        .map_err(|e| IpcError::Command(e.to_string()))
}

// ---- command wrappers ----

/// Returns the document state, falling back to the default on any IPC error.
#[cfg(not(feature = "web"))]
pub async fn get_doc_state() -> DocState {
    let result = raw_invoke("get_doc_state", JsValue::NULL).await;
    from_value(result).unwrap_or_default()
}

/// Returns the document state from the thread-local `AppState`.
#[cfg(feature = "web")]
pub async fn get_doc_state() -> DocState {
    APP_STATE.with(|s| core::get_doc_state(&s.borrow()))
}

/// Returns the restored designer state, or `None` if no puzzle is loaded.
#[cfg(not(feature = "web"))]
pub async fn get_puzzle() -> Option<State> {
    let result = raw_invoke("get_puzzle", JsValue::NULL).await;
    from_value(result).unwrap_or(None)
}

/// Returns a snapshot of the thread-local `AppState`, or `None` if no puzzle is
/// loaded. In the web build this is `None` on startup, which drives the
/// mandatory first-launch "New puzzle" modal.
#[cfg(feature = "web")]
pub async fn get_puzzle() -> Option<State> {
    APP_STATE.with(|s| core::get_puzzle(&s.borrow()))
}

#[cfg(not(feature = "web"))]
pub async fn new_latin_square(n: usize) -> Result<State, IpcError> {
    call("new_latin_square", NewPuzzleArgs { n }).await
}

#[cfg(feature = "web")]
pub async fn new_latin_square(n: usize) -> Result<State, IpcError> {
    with_state(|s| core::new_latin_square(s, n, &mut rand::rng()))
}

#[cfg(not(feature = "web"))]
pub async fn new_empty(n: usize) -> Result<State, IpcError> {
    call("new_empty", NewPuzzleArgs { n }).await
}

#[cfg(feature = "web")]
pub async fn new_empty(n: usize) -> Result<State, IpcError> {
    with_state(|s| core::new_empty(s, n))
}

#[cfg(not(feature = "web"))]
pub async fn save_puzzle(path: String) -> Result<SaveResult, IpcError> {
    call("save_puzzle", PathArgs { path }).await
}

/// Web build: the Save menu item is hidden (see [`crate::web_shims`]), so this
/// wrapper is unreachable. There is no filesystem to write to, so it reports a
/// clear error rather than silently succeeding.
#[cfg(feature = "web")]
pub async fn save_puzzle(_path: String) -> Result<SaveResult, IpcError> {
    Err(IpcError::Command(
        "saving is unavailable in the web preview".to_owned(),
    ))
}

#[cfg(not(feature = "web"))]
pub async fn load_puzzle(path: String) -> Result<State, IpcError> {
    call("load_puzzle", PathArgs { path }).await
}

/// Web build: the Open menu item is hidden and [`open_puzzle_dialog`] returns
/// `None`, so the load flow short-circuits before reaching this wrapper. The
/// body exists only to satisfy the shared signature.
#[cfg(feature = "web")]
pub async fn load_puzzle(_path: String) -> Result<State, IpcError> {
    Err(IpcError::Command(
        "opening files is unavailable in the web preview".to_owned(),
    ))
}

#[cfg(not(feature = "web"))]
pub async fn set_active_cell(active: Cell) -> Result<(), IpcError> {
    call_unit("set_active_cell", ActiveArgs { active }).await
}

#[cfg(feature = "web")]
#[allow(clippy::unnecessary_wraps)] // mirrors the fallible Tauri signature
pub async fn set_active_cell(active: Cell) -> Result<(), IpcError> {
    APP_STATE.with(|s| core::set_active_cell(&mut s.borrow_mut(), active));
    Ok(())
}

#[cfg(not(feature = "web"))]
pub async fn insert_cage(
    cells: Vec<Cell>,
    operator: Operator,
    target: Option<Target>,
) -> Result<State, IpcError> {
    call(
        "insert_cage",
        InsertCageArgs {
            cells,
            operator,
            target,
        },
    )
    .await
}

#[cfg(feature = "web")]
pub async fn insert_cage(
    cells: Vec<Cell>,
    operator: Operator,
    target: Option<Target>,
) -> Result<State, IpcError> {
    with_state(|s| core::insert_cage(s, &cells, operator, target))
}

#[cfg(not(feature = "web"))]
pub async fn remove_cage_at(polyomino: Polyomino) -> Result<State, IpcError> {
    call("remove_cage_at", RemoveCageAtArgs { polyomino }).await
}

#[cfg(feature = "web")]
pub async fn remove_cage_at(polyomino: Polyomino) -> Result<State, IpcError> {
    with_state(|s| core::remove_cage_at(s, &polyomino))
}

/// Snapshots the unique completion into the solution (Without-Solution →
/// With-Solution). Errors if the puzzle does not have exactly one completion.
#[cfg(not(feature = "web"))]
pub async fn fix() -> Result<State, IpcError> {
    call_no_args("fix").await
}

#[cfg(feature = "web")]
pub async fn fix() -> Result<State, IpcError> {
    with_state(core::fix)
}

/// Discards the solution (With-Solution → Without-Solution).
#[cfg(not(feature = "web"))]
pub async fn unfix() -> Result<State, IpcError> {
    call_no_args("unfix").await
}

#[cfg(feature = "web")]
pub async fn unfix() -> Result<State, IpcError> {
    with_state(core::unfix)
}

#[cfg(not(feature = "web"))]
pub async fn set_window_title(title: String) -> Result<(), IpcError> {
    call_unit("set_window_title", TitleArgs { title }).await
}

/// Web build: writes the browser tab title via `document.title`.
#[cfg(feature = "web")]
pub async fn set_window_title(title: String) -> Result<(), IpcError> {
    crate::web_shims::set_window_title(&title);
    Ok(())
}

/// Exits the application. Never returns meaningfully (the process is killed).
#[cfg(not(feature = "web"))]
pub async fn quit_app() {
    raw_invoke("quit_app", JsValue::NULL).await;
}

/// Web build: there is no application to quit, and the Quit menu item is hidden,
/// so this is a no-op.
#[cfg(feature = "web")]
pub async fn quit_app() {}

// ---- file dialogs ----

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct FileFilter {
    name: String,
    extensions: Vec<String>,
}

#[cfg(not(feature = "web"))]
#[derive(Serialize)]
struct DialogOptions {
    filters: Vec<FileFilter>,
}

#[cfg(not(feature = "web"))]
fn mathdoku_dialog_options() -> DialogOptions {
    DialogOptions {
        filters: vec![FileFilter {
            name: "Mathdoku".to_owned(),
            extensions: vec!["mathdoku".to_owned()],
        }],
    }
}

/// Opens the native "open file" dialog, returning the chosen path or `None`
/// if the user cancelled.
#[cfg(not(feature = "web"))]
pub async fn open_puzzle_dialog() -> Option<String> {
    let options = to_value(&mathdoku_dialog_options()).ok()?;
    dialog_open(options).await.as_string()
}

/// Web build: no native file dialog exists; the Open menu item is hidden, so
/// this always reports "cancelled".
#[cfg(feature = "web")]
pub async fn open_puzzle_dialog() -> Option<String> {
    None
}

/// Opens the native "save file" dialog, returning the chosen path or `None`
/// if the user cancelled.
#[cfg(not(feature = "web"))]
pub async fn save_puzzle_dialog() -> Option<String> {
    let options = to_value(&mathdoku_dialog_options()).ok()?;
    dialog_save(options).await.as_string()
}

/// Web build: no native file dialog exists; the Save menu item is hidden, so
/// this always reports "cancelled".
#[cfg(feature = "web")]
pub async fn save_puzzle_dialog() -> Option<String> {
    None
}
