#![allow(
    clippy::future_not_send,        // WASM async is inherently single-threaded
    clippy::items_after_statements, // use wasm_bindgen::JsCast inside async blocks
    clippy::too_many_lines,         // App component is inherently long
    unused_results,                 // listen/Effect::new return values are fire-and-forget in WASM
)]

use leptos::prelude::*;
use leptos::task::spawn_local;
use mathdoku_designer_core::State;
use wasm_bindgen::prelude::*;

use crate::components::{PendingCommit, Puzzle};
#[cfg(feature = "without-solution")]
use crate::help::NO_SOLUTION_TOOLTIP;
use crate::help::{DISCARD_TOOLTIP, RANDOM_SOLUTION_TOOLTIP, SAVE_TOOLTIP, SIZE_SELECT_TOOLTIP};
use crate::ipc;
use crate::keys::{ESCAPE, TAB};
use crate::theme::{ACCENT, BG, INK, INK2, LINE, SANS as SANS_FONT};

// ---- Tauri event glue ----
//
// Command IPC lives in `crate::ipc`; only the `listen` event-bus binding,
// which takes a JS callback, stays here.
#[cfg(not(feature = "web"))]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &js_sys::Function) -> JsValue;
}

// The web build has no Tauri event bus. Menu-driven actions either
// arrive through the first-launch modal (New) or have no web analog (Save,
// Open, Quit/close), so `listen` is a no-op here: the handlers below are still
// registered but never fire, and nothing touches `window.__TAURI__`.
#[cfg(feature = "web")]
#[allow(clippy::unused_async)] // call sites `.await` it; mirrors the Tauri binding
async fn listen(_event: &str, _handler: &js_sys::Function) -> JsValue {
    JsValue::NULL
}

/// The ephemeral-demo banner: rendered above the canvas in the web build,
/// nothing at all in the Tauri build.
#[cfg(feature = "web")]
fn ephemeral_banner() -> impl IntoView {
    view! { <crate::web_state::EphemeralBanner /> }
}

#[cfg(not(feature = "web"))]
fn ephemeral_banner() -> impl IntoView {}

/// Saves the current puzzle. `Ok(None)` means the user cancelled the save
/// dialog, `Ok(Some(path))` means the write succeeded, and `Err(e)` means the
/// write failed and the caller must surface the error.
async fn call_save_puzzle() -> Result<Option<String>, ipc::IpcError> {
    let state = ipc::get_doc_state().await;
    let path = match state.path {
        Some(p) => Some(p),
        None => ipc::save_puzzle_dialog().await,
    };
    if let Some(path) = path {
        ipc::save_puzzle(path.clone()).await?;
        return Ok(Some(path));
    }
    Ok(None)
}

async fn call_save_as_puzzle() -> Result<Option<String>, ipc::IpcError> {
    if let Some(path) = ipc::save_puzzle_dialog().await {
        ipc::save_puzzle(path.clone()).await?;
        return Ok(Some(path));
    }
    Ok(None)
}

async fn call_load_puzzle() -> Result<Option<State>, String> {
    let Some(path) = ipc::open_puzzle_dialog().await else {
        return Ok(None); // user cancelled the dialog
    };
    ipc::load_puzzle(path)
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

fn basename(path: &str) -> &str {
    path.rsplit(&['/', '\\']).next().unwrap_or(path)
}

/// Download progress as a 0–100 percentage, or `None` when the total size is
/// unknown (the Updating bar then renders as indeterminate). Clamps to the
/// total so a server that over-reports can't drive the bar past full.
#[cfg(not(feature = "web"))]
#[allow(clippy::cast_precision_loss)] // download sizes are far below f64's 2^52 exact range
fn progress_percent(downloaded: u64, total: Option<u64>) -> Option<f64> {
    match total {
        Some(t) if t > 0 => Some(downloaded.min(t) as f64 / t as f64 * 100.0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        basename, body_style, dialog_style, neutral_btn_style, overlay_style, primary_btn_style,
        title_style,
    };

    #[test]
    fn unix_path() {
        assert_eq!(basename("/home/user/puzzle.mathdoku"), "puzzle.mathdoku");
    }

    #[test]
    fn windows_path() {
        assert_eq!(
            basename(r"C:\Users\user\puzzle.mathdoku"),
            "puzzle.mathdoku"
        );
    }

    #[test]
    fn bare_filename() {
        assert_eq!(basename("puzzle.mathdoku"), "puzzle.mathdoku");
    }

    #[test]
    fn empty_string() {
        assert_eq!(basename(""), "");
    }

    #[test]
    fn basename_trailing_separator_is_empty() {
        assert_eq!(basename("/home/user/"), "");
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn progress_percent_is_none_without_a_known_total() {
        use super::progress_percent;
        assert_eq!(progress_percent(100, None), None);
        assert_eq!(progress_percent(100, Some(0)), None);
    }

    #[cfg(not(feature = "web"))]
    #[test]
    fn progress_percent_scales_and_clamps_to_full() {
        use super::progress_percent;
        assert_eq!(progress_percent(0, Some(200)), Some(0.0));
        assert_eq!(progress_percent(50, Some(200)), Some(25.0));
        assert_eq!(progress_percent(200, Some(200)), Some(100.0));
        // An over-reporting server can't push the bar past full.
        assert_eq!(progress_percent(500, Some(200)), Some(100.0));
    }

    #[test]
    fn overlay_style_is_a_fixed_fullscreen_overlay() {
        let s = overlay_style();
        assert!(s.contains("position:fixed"));
        assert!(s.contains("z-index:2000"));
    }

    #[test]
    fn dialog_style_embeds_width_bounds() {
        let s = dialog_style(280, 380);
        assert!(s.contains("min-width:280px"));
        assert!(s.contains("max-width:380px"));
    }

    #[test]
    fn text_styles_are_non_empty() {
        assert!(title_style().contains("font-size"));
        assert!(body_style().contains("font-size"));
    }

    #[test]
    fn primary_and_neutral_buttons_share_appearance() {
        // primary_btn_style is documented to match neutral_btn_style.
        assert_eq!(primary_btn_style(), neutral_btn_style());
        assert!(neutral_btn_style().contains("cursor:pointer"));
    }
}

// ---- modal styles ----

const fn overlay_style() -> &'static str {
    "position:fixed;inset:0;background:rgba(0,0,0,0.35);z-index:2000;\
     display:flex;align-items:center;justify-content:center;"
}

fn dialog_style(min_w: u32, max_w: u32) -> String {
    format!(
        "background:{BG};border:0.5px solid {LINE};border-radius:8px;\
         box-shadow:0 4px 24px rgba(0,0,0,0.2);padding:24px 28px;\
         font-family:{SANS_FONT};min-width:{min_w}px;max-width:{max_w}px;"
    )
}

fn title_style() -> String {
    format!("font-size:16px;font-weight:600;color:{INK};margin:0 0 10px 0;")
}

fn body_style() -> String {
    format!("font-size:13.5px;color:{INK2};margin:0 0 16px 0;")
}

fn neutral_btn_style() -> String {
    format!(
        "padding:6px 16px;border:0.5px solid {LINE};border-radius:5px;\
         background:{BG};color:{INK};font-family:{SANS_FONT};font-size:13px;cursor:pointer;"
    )
}

fn primary_btn_style() -> String {
    // Same appearance as neutral; focus ring distinguishes keyboard focus.
    neutral_btn_style()
}

/// Tab trap shared by the modal dialogs: intercepts Tab/Shift-Tab on the
/// dialog so focus cycles through the dialog's `<select>`/`<button>` controls
/// (DOM order matches Tab order) and never escapes to the grid SVG behind the
/// overlay. Attach with `on:keydown` to a dialog container that has
/// `tabindex="-1"` so the container can receive the keydown event.
fn trap_tab(ev: &leptos::ev::KeyboardEvent) {
    if ev.key() != TAB {
        return;
    }
    use wasm_bindgen::JsCast;
    let dialog = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok());
    let Some(dialog) = dialog else { return };
    let focusable = dialog
        .query_selector_all("select, button")
        .ok()
        .map(|nl| {
            (0..nl.length())
                .filter_map(|i| nl.item(i)?.dyn_into::<web_sys::HtmlElement>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if focusable.is_empty() {
        return;
    }
    let doc = web_sys::window().and_then(|w| w.document());
    let active = doc.and_then(|d| d.active_element());
    let current_idx = active.and_then(|a| {
        focusable
            .iter()
            .position(|el| el.is_same_node(Some(a.as_ref())))
    });
    ev.prevent_default();
    let len = focusable.len();
    let next = if ev.shift_key() {
        current_idx.map_or(len - 1, |i| if i == 0 { len - 1 } else { i - 1 })
    } else {
        current_idx.map_or(0, |i| (i + 1) % len)
    };
    let _ = focusable[next].focus();
}

// ---- SizeModal ----

// With-solution-only builds have a single creation path, so the modal copy
// doesn't mention authoring modes, the lone creation button is just "Create",
// and the Size row is centered under it. Builds with the `without-solution`
// feature keep the two-button layout and its explanatory copy.
#[cfg(feature = "without-solution")]
const SIZE_MODAL_BODY: &str = "Choose a grid size, then how to author it.";
#[cfg(not(feature = "without-solution"))]
const SIZE_MODAL_BODY: &str = "Choose a grid size.";

#[cfg(feature = "without-solution")]
const CREATE_BTN_LABEL: &str = "Random Solution";
#[cfg(not(feature = "without-solution"))]
const CREATE_BTN_LABEL: &str = "Create";

#[cfg(feature = "without-solution")]
const SIZE_ROW_STYLE: &str = "display:flex;align-items:center;gap:8px;margin-bottom:20px;";
#[cfg(not(feature = "without-solution"))]
const SIZE_ROW_STYLE: &str =
    "display:flex;align-items:center;justify-content:center;gap:8px;margin-bottom:20px;";

#[component]
fn SizeModal(
    default_n: usize,
    /// Creates a With-Solution puzzle (random Latin square) of the chosen size.
    on_create_with_solution: Callback<usize>,
    /// Creates an empty Without-Solution puzzle of the chosen size. Only
    /// supplied by builds with the `without-solution` feature; without it the
    /// modal offers no "No Solution" button.
    #[prop(optional)]
    on_create_empty: Option<Callback<usize>>,
    on_cancel: Callback<()>,
    /// When true, Escape, backdrop click, and the Cancel button are all disabled.
    /// Used on first launch when no puzzle exists yet.
    #[prop(default = false)]
    mandatory: bool,
) -> impl IntoView {
    let chosen = RwSignal::new(default_n);
    let select_style = format!(
        "padding:4px 8px;border:0.5px solid {LINE};border-radius:4px;\
         font-family:{SANS_FONT};font-size:13px;background:{BG};color:{INK};"
    );

    // Move focus to the Size dropdown as soon as the modal mounts. The
    // `autofocus` attribute only applies during initial document load, so a
    // modal opened later (e.g. Cmd-N) would leave focus on the grid behind the
    // overlay — outside the Tab trap below.
    let select_ref = NodeRef::<leptos::html::Select>::new();
    Effect::new(move |_| {
        if let Some(el) = select_ref.get() {
            let _ = el.focus();
        }
    });

    let _esc = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() == ESCAPE && !mandatory {
            on_cancel.run(());
        }
    });

    // The Without-Solution creation path. Renders nothing when the
    // `without-solution` feature is off — the modal then offers only
    // Create (and Cancel).
    #[cfg(feature = "without-solution")]
    let no_solution_btn = on_create_empty.map_or_else(
        || ().into_any(),
        |cb| {
            let style = neutral_btn_style();
            view! {
                <button
                    class="sz-btn"
                    style=style
                    title=NO_SOLUTION_TOOLTIP
                    on:click=move |_| cb.run(chosen.get_untracked())
                >
                    "No Solution"
                </button>
            }
            .into_any()
        },
    );
    #[cfg(not(feature = "without-solution"))]
    let no_solution_btn = {
        let _ = &on_create_empty;
        ().into_any()
    };

    view! {
        <div
            style=overlay_style()
            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                if !mandatory && ev.target() == ev.current_target() { on_cancel.run(()); }
            }
        >
            // `tabindex="-1"` lets this div receive the keydown event for the trap.
            <div style=dialog_style(280, 380) tabindex="-1" on:keydown=|ev| trap_tab(&ev)>
                // Focus ring for buttons: inline styles cannot express :focus-visible,
                // so a scoped <style> block provides it.
                <style>
                    ".sz-btn:focus-visible { outline: 2px solid "
                    {ACCENT}
                    "; outline-offset: 2px; }"
                </style>
                <p style=title_style()>"New puzzle"</p>
                <p style=body_style()>{SIZE_MODAL_BODY}</p>
                <div style=SIZE_ROW_STYLE>
                    <label style=format!("font-size:13px;color:{INK};")>
                        "Size: "
                        <select
                            node_ref=select_ref
                            style=select_style
                            title=SIZE_SELECT_TOOLTIP
                            on:change=move |ev: leptos::ev::Event| {
                                if let Ok(n) = event_target_value(&ev).parse::<usize>() {
                                    chosen.set(n);
                                }
                            }
                            prop:value=move || chosen.get().to_string()
                        >
                            <option value="2">"2"</option>
                            <option value="3">"3"</option>
                            <option value="4">"4"</option>
                            <option value="5">"5"</option>
                            <option value="6">"6"</option>
                            <option value="7">"7"</option>
                            <option value="8">"8"</option>
                            <option value="9">"9"</option>
                        </select>
                    </label>
                </div>
                <div style="display:flex;justify-content:center;gap:10px;">
                    {no_solution_btn}
                    <button
                        class="sz-btn"
                        style=primary_btn_style()
                        title=RANDOM_SOLUTION_TOOLTIP
                        on:click=move |_| on_create_with_solution.run(chosen.get_untracked())
                    >
                        {CREATE_BTN_LABEL}
                    </button>
                    {(!mandatory).then(||
                        view! {
                            <button class="sz-btn" style=neutral_btn_style() on:click=move |_| on_cancel.run(())>
                                "Cancel"
                            </button>
                        }
                    )}
                </div>
            </div>
        </div>
    }
}

// ---- UnsavedChangesModal ----

#[component]
fn UnsavedChangesModal(
    on_save: Callback<()>,
    on_discard: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    // Focus the default (Save) button on mount; `autofocus` only applies
    // during initial document load, not when the modal mounts later.
    let save_ref = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        if let Some(el) = save_ref.get() {
            let _ = el.focus();
        }
    });

    // Escape cancels the close request, matching the backdrop click.
    let _esc = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() == ESCAPE {
            on_cancel.run(());
        }
    });

    view! {
        <div
            style=overlay_style()
            on:mousedown=move |ev: leptos::ev::MouseEvent| {
                if ev.target() == ev.current_target() { on_cancel.run(()); }
            }
        >
            // `tabindex="-1"` lets this div receive the keydown event for the trap.
            <div style=dialog_style(340, 420) tabindex="-1" on:keydown=|ev| trap_tab(&ev)>
                // Focus ring for buttons: inline styles cannot express :focus-visible,
                // so a scoped <style> block provides it.
                <style>
                    ".uc-btn:focus-visible { outline: 2px solid "
                    {ACCENT}
                    "; outline-offset: 2px; }"
                </style>
                <p style=title_style()>"Save changes before closing?"</p>
                <p style=body_style()>"This puzzle has unsaved changes."</p>
                <div style="display:flex;justify-content:flex-end;gap:10px;flex-wrap:wrap;">
                    <button
                        class="uc-btn"
                        style=neutral_btn_style()
                        title=DISCARD_TOOLTIP
                        on:click=move |_| on_discard.run(())
                    >
                        "Don\u{2019}t Save"
                    </button>
                    <button class="uc-btn" style=neutral_btn_style() on:click=move |_| on_cancel.run(())>
                        "Cancel"
                    </button>
                    <button
                        class="uc-btn"
                        node_ref=save_ref
                        style=primary_btn_style()
                        title=SAVE_TOOLTIP
                        on:click=move |_| on_save.run(())
                    >
                        "Save"
                    </button>
                </div>
            </div>
        </div>
    }
}

// ---- ErrorToast ----

#[component]
fn ErrorToast(message: String, on_dismiss: Callback<()>) -> impl IntoView {
    // Focus the OK button on mount; `autofocus` only applies during initial
    // document load, not when the toast mounts later.
    let ok_ref = NodeRef::<leptos::html::Button>::new();
    Effect::new(move |_| {
        if let Some(el) = ok_ref.get() {
            let _ = el.focus();
        }
    });
    let toast_style = format!(
        "background:{BG};border:0.5px solid {LINE};border-radius:8px;\
         box-shadow:0 4px 24px rgba(0,0,0,0.2);padding:20px 24px;\
         font-family:{SANS_FONT};min-width:300px;max-width:480px;"
    );
    view! {
        <div style=overlay_style()>
            // `tabindex="-1"` lets this div receive the keydown event for the trap.
            <div style=toast_style tabindex="-1" on:keydown=|ev| trap_tab(&ev)>
                <p style=title_style()>"Error"</p>
                <p style=body_style()>{message}</p>
                <div style="display:flex;justify-content:flex-end;">
                    <button
                        node_ref=ok_ref
                        style=primary_btn_style()
                        on:click=move |_| on_dismiss.run(())
                    >
                        "OK"
                    </button>
                </div>
            </div>
        </div>
    }
}

// ---- UpdatingModal ----

// The progress UI for the self-update flow. Tauri ships no updater UI, so the
// app builds its own: while a newer release downloads this modal shows a
// progress bar (advancing when the server announces a size, indeterminate when
// it doesn't), then the app installs and relaunches. If the check, download, or
// install fails, `error` is set and the modal swaps to a dismissible message so
// the failure is never swallowed. The browser preview has no self-update, so
// the whole component is Tauri-only.
#[cfg(not(feature = "web"))]
#[component]
fn UpdatingModal(
    /// Running downloaded byte count, fed by the `update://progress` event.
    downloaded: RwSignal<u64>,
    /// Total download size, or `None` for an indeterminate bar.
    total: RwSignal<Option<u64>>,
    /// `Some(msg)` switches the modal from progress to a dismissible error.
    error: RwSignal<Option<String>>,
    /// Dismisses the modal; only reachable from the error state.
    on_dismiss: Callback<()>,
) -> impl IntoView {
    use crate::theme::ACCENT;

    let track_style = format!(
        "height:8px;border-radius:4px;background:{LINE};overflow:hidden;margin:4px 0 4px 0;"
    );
    // The fill animates its width so a determinate bar glides between updates;
    // an indeterminate bar (unknown total) just shows a full accent track.
    let fill_style = move || {
        let width = progress_percent(downloaded.get(), total.get()).unwrap_or(100.0);
        format!(
            "height:100%;width:{width}%;background:{ACCENT};\
             transition:width 0.2s ease;"
        )
    };
    let status = move || {
        progress_percent(downloaded.get(), total.get()).map_or_else(
            || "Downloading update…".to_string(),
            |pct| format!("Downloading update… {pct:.0}%"),
        )
    };

    view! {
        <div style=overlay_style()>
            <div style=dialog_style(320, 420) tabindex="-1" on:keydown=|ev| trap_tab(&ev)>
                <style>
                    ".upd-btn:focus-visible { outline: 2px solid "
                    {ACCENT}
                    "; outline-offset: 2px; }"
                </style>
                {move || error.get().map_or_else(
                    // Progress state: title, animated bar, percentage text.
                    || view! {
                        <p style=title_style()>"Updating Mathdoku Designer"</p>
                        <p style=body_style()>{status}</p>
                        <div style=track_style.clone()>
                            <div style=fill_style></div>
                        </div>
                        <p style=format!("font-size:12px;color:{INK2};margin:8px 0 0 0;")>
                            "The app will relaunch automatically when the update is installed."
                        </p>
                    }.into_any(),
                    // Error state: message plus a dismiss button.
                    |msg| view! {
                        <p style=title_style()>"Update failed"</p>
                        <p style=body_style()>{msg}</p>
                        <div style="display:flex;justify-content:flex-end;">
                            <button
                                class="upd-btn"
                                style=primary_btn_style()
                                on:click=move |_| on_dismiss.run(())
                            >
                                "Dismiss"
                            </button>
                        </div>
                    }.into_any(),
                )}
            </div>
        </div>
    }
}

/// Renders the Updating modal when `show` is set (Tauri build only).
#[cfg(not(feature = "web"))]
fn update_modal_view(
    show: RwSignal<bool>,
    downloaded: RwSignal<u64>,
    total: RwSignal<Option<u64>>,
    error: RwSignal<Option<String>>,
    on_dismiss: Callback<()>,
) -> impl IntoView {
    move || {
        show.get().then(|| {
            view! {
                <UpdatingModal
                    downloaded=downloaded
                    total=total
                    error=error
                    on_dismiss=on_dismiss
                />
            }
        })
    }
}

/// Web build: the browser preview has no self-update, so this renders nothing.
#[cfg(feature = "web")]
fn update_modal_view(
    show: RwSignal<bool>,
    downloaded: RwSignal<u64>,
    total: RwSignal<Option<u64>>,
    error: RwSignal<Option<String>>,
    on_dismiss: Callback<()>,
) -> impl IntoView {
    let _ = (show, downloaded, total, error, on_dismiss);
}

// ---- App ----

#[component]
pub fn App() -> impl IntoView {
    let show_size_modal = RwSignal::new(false);
    let show_unsaved_modal = RwSignal::new(false);
    let error_msg: RwSignal<Option<String>> = RwSignal::new(None);
    let designer_state = RwSignal::new(None::<State>);
    // The state displayed before the most recent puzzle change. The Puzzle
    // diffs it against the new state to flash candidates the change removed;
    // `None` (first mount, New, Open) suppresses the flash.
    let previous_state: RwSignal<Option<State>> = RwSignal::new(None);
    let current_path: RwSignal<Option<String>> = RwSignal::new(None);
    let undo_stack: RwSignal<Vec<State>> = RwSignal::new(Vec::new());
    let redo_stack: RwSignal<Vec<State>> = RwSignal::new(Vec::new());
    let pending_commit: RwSignal<Option<PendingCommit>> = RwSignal::new(None);
    // After a cage is demoted, the new Puzzle instance reads this signal on
    // mount and opens its own operation selector for the polyomino. Using a
    // signal (rather than a Callback from the old Puzzle) ensures the open
    // happens in the new Puzzle's scope, not the disposed old one.
    let pending_selector: RwSignal<Option<mathdoku::Polyomino>> = RwSignal::new(None);

    // Self-update modal state. `show_update_modal` gates the Updating modal;
    // the byte counts drive its progress bar; `update_error` swaps it to a
    // dismissible error. These are read by `update_modal_view` on every build
    // (the web variant ignores them) and driven by the Tauri-only check flow.
    let show_update_modal = RwSignal::new(false);
    let update_downloaded = RwSignal::new(0_u64);
    let update_total: RwSignal<Option<u64>> = RwSignal::new(None);
    let update_error: RwSignal<Option<String>> = RwSignal::new(None);

    // Check if a puzzle was already restored from the recent file on startup.
    // If not, show the Size Modal so the user can create a new puzzle.
    spawn_local(async move {
        if let Some(st) = ipc::get_puzzle().await {
            let ds = ipc::get_doc_state().await;
            current_path.set(ds.path);
            designer_state.set(Some(st));
        } else {
            show_size_modal.set(true);
        }
    });

    // Drive the self-update check from the frontend on mount (issue #172). The
    // progress listener is registered *before* the check so no `update://progress`
    // chunk event can fire before the webview is listening. Tauri's updater never
    // runs on its own; this is the call that makes it go. Gated to the Tauri build
    // — the browser preview has no command bus or self-update.
    #[cfg(not(feature = "web"))]
    spawn_local(async move {
        let progress_cb = Closure::wrap(Box::new(move |event: JsValue| {
            if let Some(p) = ipc::parse_update_progress(&event) {
                update_downloaded.set(p.downloaded);
                update_total.set(p.total);
            }
        }) as Box<dyn Fn(JsValue)>);
        // Event names must match the EVENT_UPDATE_* constants in src-tauri/src/commands.rs.
        listen("update://progress", progress_cb.as_ref().unchecked_ref()).await;
        progress_cb.forget();

        match ipc::check_for_update().await {
            Ok(check) if check.available => {
                // Auto-start the download (a pre-install confirmation can come
                // later, per the issue). Open the modal, then install: any
                // error from the install drives the modal's error state.
                update_downloaded.set(0);
                update_total.set(None);
                update_error.set(None);
                show_update_modal.set(true);
                if let Err(e) = ipc::install_update().await {
                    update_error.set(Some(e.to_string()));
                }
                // On success the process restarts and never reaches here.
            }
            Ok(_) => {} // up to date: no modal, startup unaffected
            Err(e) => {
                // Don't fail silently: surface check/permission errors too.
                update_error.set(Some(e.to_string()));
                show_update_modal.set(true);
            }
        }
    });

    spawn_local(async move {
        let new_cb = Closure::wrap(Box::new(move |_: JsValue| {
            show_size_modal.set(true);
        }) as Box<dyn Fn(JsValue)>);
        // Event names must match the EVENT_* constants in src-tauri/src/lib.rs.
        listen("menu-new", new_cb.as_ref().unchecked_ref()).await;
        new_cb.forget();

        #[allow(clippy::type_complexity)]
        let make_save_cb = move |fut_fn: fn() -> std::pin::Pin<
            Box<dyn Future<Output = Result<Option<String>, ipc::IpcError>>>,
        >| {
            Closure::wrap(Box::new(move |_: JsValue| {
                spawn_local(async move {
                    match fut_fn().await {
                        Ok(Some(path)) => current_path.set(Some(path)),
                        Ok(None) => {} // user cancelled dialog
                        Err(e) => error_msg.set(Some(e.to_string())),
                    }
                });
            }) as Box<dyn Fn(JsValue)>)
        };
        let save_cb = make_save_cb(|| Box::pin(call_save_puzzle()));
        listen("menu-save", save_cb.as_ref().unchecked_ref()).await;
        save_cb.forget();

        let save_as_cb = make_save_cb(|| Box::pin(call_save_as_puzzle()));
        listen("menu-save-as", save_as_cb.as_ref().unchecked_ref()).await;
        save_as_cb.forget();

        let load_cb = Closure::wrap(Box::new(move |_: JsValue| {
            spawn_local(async move {
                match call_load_puzzle().await {
                    Ok(Some(st)) => {
                        let ds = ipc::get_doc_state().await;
                        current_path.set(ds.path);
                        undo_stack.update(Vec::clear);
                        redo_stack.update(Vec::clear);
                        pending_commit.set(None);
                        previous_state.set(None);
                        designer_state.set(Some(st));
                        // Loading a puzzle satisfies the New Puzzle condition, so
                        // dismiss the size modal if it is open (including the
                        // mandatory first-launch case).
                        show_size_modal.set(false);
                    }
                    Ok(None) => {} // user cancelled dialog
                    Err(e) => error_msg.set(Some(e)),
                }
            });
        }) as Box<dyn Fn(JsValue)>);
        listen("menu-open", load_cb.as_ref().unchecked_ref()).await;
        load_cb.forget();

        // menu-fix / menu-unfix: the native Puzzle menu's accelerators
        // (CmdOrCtrl+L / CmdOrCtrl+Shift+L) arrive here. Both delegate to
        // ipc::fix / ipc::unfix and push the prior state onto the undo stack like
        // any other puzzle change. The mode predicate is re-checked defensively so
        // a stale menu event can't drive an invalid transition. (On web `listen`
        // is a no-op and these never fire — the in-page keydown handler drives
        // Fix/Unfix there instead.) The Puzzle menu only exists with the
        // `without-solution` feature.
        #[cfg(feature = "without-solution")]
        {
            #[allow(clippy::type_complexity)]
            let make_mode_cb = move |needs_solution: bool,
                                     fut_fn: fn() -> std::pin::Pin<
                Box<dyn Future<Output = Result<State, ipc::IpcError>>>,
            >| {
                Closure::wrap(Box::new(move |_: JsValue| {
                    if designer_state
                        .get_untracked()
                        .is_some_and(|st| st.solution.is_some() == needs_solution)
                    {
                        spawn_local(async move {
                            match fut_fn().await {
                                Ok(new_st) => {
                                    if let Some(pre) = designer_state.get_untracked() {
                                        undo_stack.update(|s| s.push(pre));
                                    }
                                    redo_stack.update(Vec::clear);
                                    previous_state.set(designer_state.get_untracked());
                                    designer_state.set(Some(new_st));
                                }
                                Err(e) => error_msg.set(Some(e.to_string())),
                            }
                        });
                    }
                }) as Box<dyn Fn(JsValue)>)
            };
            // Fix requires Without-Solution mode (solution.is_some() == false);
            // Unfix requires With-Solution mode (solution.is_some() == true).
            let fix_cb = make_mode_cb(false, || Box::pin(ipc::fix()));
            listen("menu-fix", fix_cb.as_ref().unchecked_ref()).await;
            fix_cb.forget();
            let unfix_cb = make_mode_cb(true, || Box::pin(ipc::unfix()));
            listen("menu-unfix", unfix_cb.as_ref().unchecked_ref()).await;
            unfix_cb.forget();
        }

        let close_cb = Closure::wrap(Box::new(move |_: JsValue| {
            show_unsaved_modal.set(true);
        }) as Box<dyn Fn(JsValue)>);
        listen("request-close", close_cb.as_ref().unchecked_ref()).await;
        close_cb.forget();
    });

    // Both creation paths share the same post-create bookkeeping; they differ
    // only in which Tauri command builds the initial State.
    let install_new_state = move |result: Result<State, ipc::IpcError>| match result {
        Ok(st) => {
            current_path.set(None);
            undo_stack.update(Vec::clear);
            redo_stack.update(Vec::clear);
            pending_commit.set(None);
            previous_state.set(None);
            designer_state.set(Some(st));
        }
        Err(e) => error_msg.set(Some(e.to_string())),
    };
    let on_create_with_solution = Callback::new(move |n: usize| {
        show_size_modal.set(false);
        spawn_local(async move { install_new_state(ipc::new_latin_square(n).await) });
    });
    #[cfg(feature = "without-solution")]
    let on_create_empty = Callback::new(move |n: usize| {
        show_size_modal.set(false);
        spawn_local(async move { install_new_state(ipc::new_empty(n).await) });
    });
    let on_create_cancel = Callback::new(move |(): ()| show_size_modal.set(false));

    let on_unsaved_save = Callback::new(move |(): ()| {
        show_unsaved_modal.set(false);
        spawn_local(async move {
            match call_save_puzzle().await {
                Ok(_) => ipc::quit_app().await,
                Err(e) => error_msg.set(Some(e.to_string())),
            }
        });
    });
    let on_unsaved_discard = Callback::new(move |(): ()| {
        show_unsaved_modal.set(false);
        spawn_local(async move { ipc::quit_app().await });
    });
    let on_unsaved_cancel = Callback::new(move |(): ()| show_unsaved_modal.set(false));

    Effect::new(move |_| {
        let title = current_path
            .get()
            .map(|p| basename(&p).to_owned())
            .unwrap_or_default();
        spawn_local(async move {
            let _ = ipc::set_window_title(title).await;
        });
    });

    let on_dismiss_error = Callback::new(move |(): ()| error_msg.set(None));

    // Dismissing the Updating modal only happens from its error state: clear the
    // message and close it. (A successful update restarts the app, so the
    // progress state is never dismissed by hand.)
    let on_update_dismiss = Callback::new(move |(): ()| {
        update_error.set(None);
        show_update_modal.set(false);
    });
    let update_modal = update_modal_view(
        show_update_modal,
        update_downloaded,
        update_total,
        update_error,
        on_update_dismiss,
    );

    // These callbacks are stable for the app's lifetime and must not be
    // re-created inside the reactive closure below. Puzzle instances capture
    // them in async closures (demote → commit chains) that outlive the Puzzle
    // that created them; if the callback were tied to a reactive scope that
    // gets disposed on re-mount, calling it from a stale async closure would
    // silently do nothing and the re-mount would never fire.
    let on_puzzle_change = Callback::new(move |new_st: State| {
        previous_state.set(designer_state.get_untracked());
        designer_state.set(Some(new_st));
    });
    let on_state_change = Callback::new(move |_new_st: State| {});
    let on_error = Callback::new(move |msg: String| error_msg.set(Some(msg)));

    // `on_create_empty` is only supplied by builds with the `without-solution`
    // feature, so the SizeModal instantiation is forked on the feature.
    let size_modal_view = move || {
        show_size_modal.get().then(|| {
            let default_n = designer_state.get().map_or(9, |st| st.puzzle.n());
            let mandatory = designer_state.get().is_none();
            #[cfg(feature = "without-solution")]
            {
                view! {
                    <SizeModal
                        default_n=default_n
                        on_create_with_solution=on_create_with_solution
                        on_create_empty=on_create_empty
                        on_cancel=on_create_cancel
                        mandatory=mandatory
                    />
                }
            }
            #[cfg(not(feature = "without-solution"))]
            {
                view! {
                    <SizeModal
                        default_n=default_n
                        on_create_with_solution=on_create_with_solution
                        on_cancel=on_create_cancel
                        mandatory=mandatory
                    />
                }
            }
        })
    };

    view! {
        <main class="app-main">
            {ephemeral_banner()}
            {move || designer_state.get().map(|st| {
            // Untracked: previous_state changes only alongside designer_state;
            // tracking it here would re-mount the Puzzle twice per change.
            let prev = previous_state.get_untracked();
            view! { <Puzzle state=st prev_state=prev undo_stack=undo_stack redo_stack=redo_stack pending_commit=pending_commit pending_selector=pending_selector on_puzzle_change=on_puzzle_change on_state_change=on_state_change on_error=on_error /> }
        })}
            {size_modal_view}
            {move || show_unsaved_modal.get().then(|| view! {
                <UnsavedChangesModal
                    on_save=on_unsaved_save
                    on_discard=on_unsaved_discard
                    on_cancel=on_unsaved_cancel
                />
            })}
            {move || error_msg.get().map(|msg| view! {
                <ErrorToast message=msg on_dismiss=on_dismiss_error />
            })}
            {update_modal}
        </main>
    }
}
