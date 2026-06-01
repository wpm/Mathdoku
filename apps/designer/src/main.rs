mod app;
mod cage_commit;
mod components;
pub mod feasibility;
pub mod geometry;
pub mod ipc;
pub mod keys;
pub mod partial_solution;
mod theme;
#[cfg(feature = "web")]
mod web_shims;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    // The web build owns its `AppState` in a thread-local singleton; construct
    // it before mounting so the first render sees a ready backend. The Tauri
    // build has no equivalent frontend-side startup — its state lives in the
    // backend process.
    #[cfg(feature = "web")]
    web_shims::init_app_state();
    mount_to_body(|| view! { <App /> });
}
