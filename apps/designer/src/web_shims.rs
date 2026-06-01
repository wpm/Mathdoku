//! Web-only behavior with no Tauri analogue (ADR-0002).
//!
//! The whole module compiles only under `--features web`. It owns the
//! in-browser [`AppState`] singleton, the `document.title` shim, and the
//! ephemeral-demo banner. The default (Tauri) build never sees any of it.
//!
//! `AppState` ownership intentionally differs between the two builds: the Tauri
//! backend keeps it behind a `Mutex<AppState>` in `tauri::State`, while the web
//! build keeps it in a `thread_local!<RefCell<AppState>>` here. WASM is
//! single-threaded, so the `RefCell` never contends.

use std::cell::RefCell;

use leptos::prelude::*;
use mathdoku_designer_core::AppState;

thread_local! {
    /// The in-browser document model the `ipc` wrappers mutate directly,
    /// standing in for the Tauri-managed `Mutex<AppState>` of the native build.
    pub static APP_STATE: RefCell<AppState> = RefCell::new(AppState::default());
}

/// Constructs the thread-local [`AppState`] singleton at startup.
///
/// `APP_STATE` is lazily initialized on first access, so this is really an
/// explicit, named hook called from `main` before Leptos mounts — it makes the
/// startup order match the Tauri build, where `manage(Mutex::new(...))` runs
/// before the window appears.
pub fn init_app_state() {
    APP_STATE.with(|s| {
        *s.borrow_mut() = AppState::default();
    });
}

/// Sets the browser tab title via `document.title`.
///
/// The Tauri build routes the equivalent call through `window.set_title`; here
/// the DOM is written directly. A missing `window`/`document` (impossible in a
/// real browser) is silently ignored.
pub fn set_window_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(title);
    }
}

/// A short banner rendered above the canvas, making the ephemeral nature of the
/// web demo explicit: reloading the tab starts the visitor over (ADR-0002).
#[component]
pub fn EphemeralBanner() -> impl IntoView {
    let style = "padding:6px 12px;background:#FFF7E6;border-bottom:0.5px solid #E5D8B8;\
                 color:#7A5C00;font-family:sans-serif;font-size:12.5px;text-align:center;";
    view! {
        <div style=style>
            "Ephemeral demo \u{2014} install the app to save what you make."
        </div>
    }
}
