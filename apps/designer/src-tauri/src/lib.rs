//! Tauri backend for Mathdoku Designer.
//!
//! Manages the application menu, window lifecycle, startup restore, and the
//! [`commands`] module that implements the Tauri IPC command handlers.

pub mod commands;

use std::fs;
use std::sync::Mutex;

use mathdoku::Puzzle;
use mathdoku_designer_core::{self as core, AppState};
// Only the macOS app menu (About panel) uses these.
#[cfg(target_os = "macos")]
use tauri::image::Image;
#[cfg(target_os = "macos")]
use tauri::menu::AboutMetadata;
use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Manager, Runtime, WindowEvent};

#[cfg(feature = "without-solution")]
use commands::{PuzzleMenu, fix, new_empty, set_puzzle_menu_enabled, unfix};
use commands::{
    check_for_update, get_doc_state, get_puzzle, insert_cage, install_update, load_puzzle,
    new_latin_square, quit_app, read_recent, remove_cage_at, save_puzzle, set_active_cell,
    set_window_title,
};

const EVENT_NEW: &str = "menu-new";
const EVENT_OPEN: &str = "menu-open";
const EVENT_SAVE: &str = "menu-save";
const EVENT_SAVE_AS: &str = "menu-save-as";
#[cfg(feature = "without-solution")]
const EVENT_FIX: &str = "menu-fix";
#[cfg(feature = "without-solution")]
const EVENT_UNFIX: &str = "menu-unfix";
const EVENT_REQUEST_CLOSE: &str = "request-close";

const MENU_ID_HELP_RULES: &str = "help_rules";
const MENU_ID_HELP_GUIDE: &str = "help_guide";

// ---- menu ----

/// Builds the application menu (File, Edit, Puzzle, View, Window, Help;
/// App menu on macOS).
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let file_menu = build_file_menu(app)?;
    let edit_menu = build_edit_menu(app)?;
    #[cfg(feature = "without-solution")]
    let puzzle_menu = build_puzzle_menu(app)?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    let help_menu = build_help_menu(app)?;

    #[cfg(target_os = "macos")]
    {
        let app_menu = Submenu::with_items(
            app,
            "Mathdoku Designer",
            true,
            &[
                &PredefinedMenuItem::about(
                    app,
                    Some("About Mathdoku Designer"),
                    Some(AboutMetadata {
                        icon: Some(Image::from_bytes(include_bytes!("../icons/128x128.png"))?),
                        ..Default::default()
                    }),
                )?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::services(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, None)?,
                &PredefinedMenuItem::hide_others(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::quit(app, None)?,
            ],
        )?;
        let view_menu = Submenu::with_items(
            app,
            "View",
            true,
            &[&PredefinedMenuItem::fullscreen(app, None)?],
        )?;
        let mut items: Vec<&dyn tauri::menu::IsMenuItem<R>> =
            vec![&app_menu, &file_menu, &edit_menu];
        #[cfg(feature = "without-solution")]
        items.push(&puzzle_menu);
        items.push(&view_menu);
        items.push(&window_menu);
        items.push(&help_menu);
        Menu::with_items(app, &items)
    }

    #[cfg(not(target_os = "macos"))]
    {
        let mut items: Vec<&dyn tauri::menu::IsMenuItem<R>> = vec![&file_menu, &edit_menu];
        #[cfg(feature = "without-solution")]
        items.push(&puzzle_menu);
        items.push(&window_menu);
        items.push(&help_menu);
        Menu::with_items(app, &items)
    }
}

/// Builds the File submenu (New, Open, Save, Save As, Close Window).
fn build_file_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let new = MenuItemBuilder::with_id("new", "New…")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open = MenuItemBuilder::with_id("open", "Open…")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let save = MenuItemBuilder::with_id("save", "Save")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;
    let save_as = MenuItemBuilder::with_id("save_as", "Save As…")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;
    Submenu::with_items(
        app,
        "File",
        true,
        &[
            &new,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &save,
            &save_as,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )
}

/// Builds the Edit submenu (the predefined clipboard and undo/redo items).
fn build_edit_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )
}

/// Builds the Help submenu (ADR-0007): its items open the website's
/// puzzle-rules and Designer-guide pages in the system browser. The app gets
/// tooltips, not documentation, so this menu is the in-app route to the
/// long-form content. The web preview build has no native menu bar at all,
/// so — exactly like Save and Open (ADR-0002) — these items have no web
/// analog and nothing needs hiding there.
fn build_help_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let rules = MenuItemBuilder::with_id(MENU_ID_HELP_RULES, "Puzzle Rules").build(app)?;
    let guide = MenuItemBuilder::with_id(MENU_ID_HELP_GUIDE, "Designer Guide").build(app)?;
    let help_menu = Submenu::with_items(app, "Help", true, &[&rules, &guide])?;
    // Register as the native Help menu so macOS adds its standard search field.
    #[cfg(target_os = "macos")]
    help_menu.set_as_help_menu_for_nsapp()?;
    Ok(help_menu)
}

/// Maps a Help menu item ID to the website URL it opens, or `None` for IDs
/// belonging to other menus. The URLs (and their anchor contract with the
/// website) live in [`core::help`], whose tests pin them against the site
/// sources.
fn help_url_for(id: &str) -> Option<&'static str> {
    match id {
        MENU_ID_HELP_RULES => Some(core::help::PUZZLE_RULES_URL),
        MENU_ID_HELP_GUIDE => Some(core::help::DESIGNER_GUIDE_URL),
        _ => None,
    }
}

/// Builds the Puzzle submenu (Fix / Unfix mode switching).
///
/// Both items are always visible; exactly one is enabled at a time, pushed from
/// the frontend via [`set_puzzle_menu_enabled`]. The item handles are
/// stashed in app state so that command can reach them to toggle `set_enabled`.
#[cfg(feature = "without-solution")]
fn build_puzzle_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Submenu<R>> {
    let fix = MenuItemBuilder::with_id("fix", "Fix Solution")
        .accelerator("CmdOrCtrl+L")
        .build(app)?;
    let unfix = MenuItemBuilder::with_id("unfix", "Unfix Solution")
        .accelerator("CmdOrCtrl+Shift+L")
        .build(app)?;
    let puzzle_menu = Submenu::with_items(app, "Puzzle", true, &[&fix, &unfix])?;
    let _ = app.manage(PuzzleMenu { fix, unfix });
    Ok(puzzle_menu)
}

/// Translates menu item IDs into frontend events emitted over the Tauri event bus.
///
/// Help items are handled natively instead: they open the website in the
/// system browser (ADR-0007) and never reach the frontend.
#[allow(clippy::needless_pass_by_value)]
fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    if let Some(url) = help_url_for(event.id().as_ref()) {
        // Failure to launch a browser is not actionable from here; ignore it
        // like the emit below.
        let _ = tauri_plugin_opener::open_url(url, None::<&str>);
        return;
    }
    let event_name = match event.id().as_ref() {
        "new" => EVENT_NEW,
        "open" => EVENT_OPEN,
        "save" => EVENT_SAVE,
        "save_as" => EVENT_SAVE_AS,
        #[cfg(feature = "without-solution")]
        "fix" => EVENT_FIX,
        #[cfg(feature = "without-solution")]
        "unfix" => EVENT_UNFIX,
        _ => return,
    };
    let _ = app.emit(event_name, ());
}

/// Intercepts close requests when there are unsaved changes.
///
/// Prevents the window from closing and emits `request-close` so the
/// frontend can show the Unsaved Changes modal.
fn handle_window_event<R: Runtime>(window: &tauri::Window<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };
    let app = window.app_handle();
    let dirty = app
        .try_state::<Mutex<AppState>>()
        .and_then(|s| s.lock().ok().map(|s| s.dirty))
        .unwrap_or(false);
    if dirty {
        api.prevent_close();
        let _ = app.emit(EVENT_REQUEST_CLOSE, ());
    }
}

// ---- startup ----

/// Attempts to restore the last session from the recent-file record.
///
/// Reads `last_open.json`, loads the referenced `.mathdoku` file through
/// [`core::apply_loaded`], and records the file path and last-known active
/// cell. Returns the restored [`Puzzle`] on success, or `None` if no recent
/// file exists, the file can't be read, or the save version is unsupported.
fn try_restore<R: Runtime>(app: &AppHandle<R>) -> Option<Puzzle> {
    let record = read_recent(app)?;
    let path = record.path?;
    let json = fs::read_to_string(&path).ok()?;
    let state = app.try_state::<Mutex<AppState>>()?;
    let mut s = state.lock().ok()?;
    let designer = core::apply_loaded(&mut s, &json).ok()?;
    s.path = Some(path);
    s.active = record.active;
    drop(s);
    Some(designer.puzzle)
}

// ---- run ----

/// # Panics
/// Panics if the Tauri application fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(clippy::expect_used)]
pub fn run() {
    mathdoku::init_debug_logging();
    // `generate_handler!` cannot carry `#[cfg]` on entries, so the command
    // list is forked on the feature.
    #[cfg(feature = "without-solution")]
    let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
        new_empty,
        new_latin_square,
        save_puzzle,
        load_puzzle,
        get_doc_state,
        get_puzzle,
        set_active_cell,
        set_window_title,
        quit_app,
        insert_cage,
        remove_cage_at,
        fix,
        unfix,
        set_puzzle_menu_enabled,
        check_for_update,
        install_update,
    ];
    #[cfg(not(feature = "without-solution"))]
    let handler: fn(tauri::ipc::Invoke<tauri::Wry>) -> bool = tauri::generate_handler![
        new_latin_square,
        save_puzzle,
        load_puzzle,
        get_doc_state,
        get_puzzle,
        set_active_cell,
        set_window_title,
        quit_app,
        insert_cage,
        remove_cage_at,
        check_for_update,
        install_update,
    ];
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    // The updater plugin is desktop-only; mobile app stores handle their own updates.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
    builder
        .manage(Mutex::new(AppState::default()))
        .menu(build_menu)
        .on_menu_event(handle_menu_event)
        .on_window_event(handle_window_event)
        .setup(|app| {
            // With the feature, an unrestored session parks a blank 9×9
            // Without-Solution puzzle. Without it, no puzzle is left in state,
            // so the frontend shows the mandatory New-puzzle modal instead.
            #[cfg(feature = "without-solution")]
            if try_restore(app.handle()).is_none()
                && let Ok(mut s) = app.state::<Mutex<AppState>>().lock()
            {
                s.puzzle = Some(Puzzle::new(9).expect("9 is a valid puzzle size"));
            }
            #[cfg(not(feature = "without-solution"))]
            let _ = try_restore(app.handle());
            Ok(())
        })
        .invoke_handler(handler)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::env::temp_dir;
    use std::path::Path;
    use std::sync::{MutexGuard, OnceLock};

    use serde::Serialize;
    use serde_json::{json, to_string, to_string_pretty};

    use mathdoku::Puzzle;

    use super::*;
    use commands::{load_puzzle, recent_path, save_puzzle};
    use mathdoku_designer_core::{SAVE_VERSION, SaveEnvelope};

    // Serialize tests that read/write the shared on-disk recent file.
    static RECENT_FILE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    fn recent_file_lock() -> MutexGuard<'static, ()> {
        RECENT_FILE_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap()
    }

    #[derive(Serialize)]
    struct RecentRecord {
        path: Option<String>,
    }

    fn write_recent_test(recent: &Path, puzzle_path: &str) {
        if let Some(parent) = recent.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let json = to_string(&RecentRecord {
            path: Some(puzzle_path.to_owned()),
        })
        .unwrap();
        fs::write(recent, json).unwrap();
    }

    fn mock_app() -> tauri::App<tauri::test::MockRuntime> {
        tauri::test::mock_app()
    }

    fn app_with_state() -> tauri::App<tauri::test::MockRuntime> {
        let app = mock_app();
        let _ = app.manage(Mutex::new(AppState::default()));
        app
    }

    fn app_with_puzzle(n: usize) -> tauri::App<tauri::test::MockRuntime> {
        let app = mock_app();
        let _ = app.manage(Mutex::new(AppState {
            puzzle: Some(Puzzle::new(n).unwrap()),
            ..AppState::default()
        }));
        app
    }

    // ---- save_puzzle / load_puzzle round-trip ----

    #[test]
    fn save_and_load_round_trips_puzzle() {
        let _guard = recent_file_lock();
        let app = app_with_puzzle(5);
        let path = temp_dir()
            .join("mathdoku_test_save_load.mathdoku")
            .to_str()
            .unwrap()
            .to_string();

        let result = save_puzzle(
            path.clone(),
            app.handle().clone(),
            app.state::<Mutex<AppState>>(),
        )
        .unwrap();
        assert_eq!(result.path, path);

        // Load into a fresh app state.
        let app2 = app_with_state();
        let puzzle = load_puzzle(
            path.clone(),
            app2.handle().clone(),
            app2.state::<Mutex<AppState>>(),
        )
        .unwrap();
        assert_eq!(puzzle.puzzle.n(), 5);
        let binding = app2.state::<Mutex<AppState>>();
        let s = binding.lock().unwrap();
        assert!(!s.dirty);
        assert_eq!(s.path.as_deref(), Some(path.as_str()));
        drop(s);
    }

    #[test]
    fn save_puzzle_sets_path_and_clears_dirty() {
        let _guard = recent_file_lock();
        let app = app_with_puzzle(4);
        // Mark dirty first.
        app.state::<Mutex<AppState>>().lock().unwrap().dirty = true;
        let path = temp_dir()
            .join("mathdoku_test_save_dirty.mathdoku")
            .to_str()
            .unwrap()
            .to_string();

        let _ = save_puzzle(
            path.clone(),
            app.handle().clone(),
            app.state::<Mutex<AppState>>(),
        )
        .unwrap();
        let binding = app.state::<Mutex<AppState>>();
        let s = binding.lock().unwrap();
        assert!(!s.dirty);
        assert_eq!(s.path.as_deref(), Some(path.as_str()));
        drop(s);
    }

    #[test]
    fn save_puzzle_errors_when_no_puzzle_loaded() {
        let app = app_with_state();
        let path = "/tmp/mathdoku_no_puzzle.mathdoku".to_string();
        assert!(save_puzzle(path, app.handle().clone(), app.state::<Mutex<AppState>>()).is_err());
    }

    #[test]
    fn save_puzzle_errors_on_bad_path() {
        let app = app_with_puzzle(4);
        let bad_path = "/nonexistent/dir/puzzle.mathdoku".to_string();
        assert!(
            save_puzzle(
                bad_path,
                app.handle().clone(),
                app.state::<Mutex<AppState>>()
            )
            .is_err()
        );
    }

    #[test]
    fn load_puzzle_errors_on_missing_file() {
        let app = app_with_state();
        assert!(
            load_puzzle(
                "/no/such/file.mathdoku".to_string(),
                app.handle().clone(),
                app.state::<Mutex<AppState>>(),
            )
            .is_err()
        );
    }

    #[test]
    fn load_puzzle_rejects_wrong_version() {
        let path = temp_dir()
            .join("mathdoku_test_bad_version.mathdoku")
            .to_str()
            .unwrap()
            .to_string();
        let puzzle = Puzzle::new(3).unwrap();
        let bad = json!({ "version": 99, "puzzle": puzzle });
        fs::write(&path, to_string(&bad).unwrap()).unwrap();

        let app = app_with_state();
        let err =
            load_puzzle(path, app.handle().clone(), app.state::<Mutex<AppState>>()).unwrap_err();
        assert!(err.contains("unsupported save version"));
    }

    #[test]
    fn load_puzzle_rejects_malformed_json() {
        let path = temp_dir()
            .join("mathdoku_test_malformed.mathdoku")
            .to_str()
            .unwrap()
            .to_string();
        fs::write(&path, "not json").unwrap();

        let app = app_with_state();
        assert!(load_puzzle(path, app.handle().clone(), app.state::<Mutex<AppState>>()).is_err());
    }

    // ---- try_restore ----

    #[test]
    fn try_restore_returns_none_when_no_recent_file() {
        let _guard = recent_file_lock();
        let app = app_with_state();
        if let Some(recent) = recent_path(app.handle()) {
            let _ = fs::remove_file(recent);
        }
        assert!(try_restore(app.handle()).is_none());
    }

    #[test]
    fn try_restore_loads_puzzle_from_saved_file() {
        let _guard = recent_file_lock();
        let puzzle_path = temp_dir()
            .join("mathdoku_test_restore.mathdoku")
            .to_str()
            .unwrap()
            .to_string();
        let puzzle = Puzzle::new(4).unwrap();
        let solution = Puzzle::new(4).unwrap();
        let envelope = SaveEnvelope {
            version: SAVE_VERSION,
            puzzle,
            solution: Some(solution),
        };
        fs::write(&puzzle_path, to_string_pretty(&envelope).unwrap()).unwrap();

        let app = app_with_state();
        let recent = recent_path(app.handle()).unwrap();
        write_recent_test(&recent, &puzzle_path);

        let restored = try_restore(app.handle());
        assert!(restored.is_some());
        assert_eq!(restored.unwrap().n(), 4);
        let binding = app.state::<Mutex<AppState>>();
        let s = binding.lock().unwrap();
        assert!(!s.dirty);
        assert_eq!(s.path.as_deref(), Some(puzzle_path.as_str()));
        drop(s);

        let _ = fs::remove_file(&recent);
    }

    #[test]
    fn try_restore_returns_none_for_wrong_version() {
        let _guard = recent_file_lock();
        let puzzle_path = temp_dir()
            .join("mathdoku_test_restore_bad_ver.mathdoku")
            .to_str()
            .unwrap()
            .to_string();
        let puzzle = Puzzle::new(3).unwrap();
        let bad = json!({ "version": 99, "puzzle": puzzle });
        fs::write(&puzzle_path, to_string(&bad).unwrap()).unwrap();

        let app = app_with_state();
        let recent = recent_path(app.handle()).unwrap();
        write_recent_test(&recent, &puzzle_path);

        assert!(try_restore(app.handle()).is_none());
        let _ = fs::remove_file(&recent);
    }

    // ---- handle_menu_event ----

    #[test]
    fn handle_menu_event_unknown_id_does_not_panic() {
        let app = mock_app();
        handle_menu_event(
            app.handle(),
            tauri::menu::MenuEvent {
                id: tauri::menu::MenuId::new("unknown"),
            },
        );
    }

    #[test]
    fn handle_menu_event_known_ids_emit_without_panic() {
        let app = mock_app();
        for id in ["new", "open", "save", "save_as", "fix", "unfix"] {
            handle_menu_event(
                app.handle(),
                tauri::menu::MenuEvent {
                    id: tauri::menu::MenuId::new(id),
                },
            );
        }
    }

    // ---- help menu ----

    #[test]
    fn help_ids_map_to_the_canonical_website_urls() {
        // The URL constants themselves are pinned against the site sources by
        // tests in mathdoku-designer-core; this checks the menu wiring.
        assert_eq!(
            help_url_for(MENU_ID_HELP_RULES),
            Some(core::help::PUZZLE_RULES_URL)
        );
        assert_eq!(
            help_url_for(MENU_ID_HELP_GUIDE),
            Some(core::help::DESIGNER_GUIDE_URL)
        );
    }

    #[test]
    fn non_help_ids_open_no_url() {
        // Every other menu ID must keep flowing to the event bus, not the
        // browser.
        for id in ["new", "open", "save", "save_as", "fix", "unfix", "nope"] {
            assert!(help_url_for(id).is_none());
        }
    }
}
