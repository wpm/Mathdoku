mod app;
mod cage_commit;
mod components;
// Global-feasibility queries exist only for Without-Solution authoring.
#[cfg(feature = "without-solution")]
pub mod feasibility;
pub mod geometry;
// Tooltip copy for the in-app help system (ADR-0007).
pub mod help;
pub mod ipc;
pub mod keys;
pub mod partial_solution;
mod theme;
// WASM-only in-process AppState store backing the `web` IPC bodies in `ipc`.
#[cfg(feature = "web")]
pub mod web_state;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
