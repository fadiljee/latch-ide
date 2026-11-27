#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]
//! Shell Desktop Native Tauri v2 untuk Latch IDE.
//! Menyalakan daemon Latch di latar belakang, menyuntikkan token lokal ke WebView,
//! serta menyediakan jendela utama (custom title bar) dan jendela floating control.

use std::time::Duration;

use latch_core::config::{load_latch_config, RuntimePaths};
use latch_daemon::http_server::ensure_or_load_token;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
fn latch_window_action(window: tauri::WebviewWindow, action: String) -> Result<(), String> {
    match action.as_str() {
        "minimize" => window.minimize().map_err(|e| e.to_string()),
        "toggle_maximize" => {
            if window.is_maximized().unwrap_or(false) {
                window.unmaximize().map_err(|e| e.to_string())
            } else {
                window.maximize().map_err(|e| e.to_string())
            }
        }
        "close" => window.close().map_err(|e| e.to_string()),
        "start_dragging" => window.start_dragging().map_err(|e| e.to_string()),
        other => Err(format!("unknown window action: {other}")),
    }
}

#[tauri::command]
fn latch_toggle_floating(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("floating") {
        if win.is_visible().unwrap_or(false) {
            win.hide().map_err(|e| e.to_string())?;
        } else {
            win.show().map_err(|e| e.to_string())?;
            win.set_focus().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn main() {
    let paths = RuntimePaths::resolve();
    let cfg = load_latch_config(&paths.config_file()).unwrap_or_default();
    let api_base = format!("http://{}", cfg.daemon.http_bind);
    let token = ensure_or_load_token(&paths.token_file()).unwrap_or_default();

    // Nyalakan daemon Latch di thread latar belakang (bila socket belum aktif).
    let daemon_paths = paths.clone();
    std::thread::spawn(move || {
        let _ = latch_daemon::run_foreground(daemon_paths);
    });
    std::thread::sleep(Duration::from_millis(250));

    let init_script = format!(
        "window.__LATCH_API_BASE__ = {}; window.__LATCH_TOKEN__ = {}; window.__LATCH_TAURI__ = true;",
        serde_json::to_string(&api_base).unwrap_or_else(|_| "\"http://127.0.0.1:7878\"".into()),
        serde_json::to_string(&token).unwrap_or_else(|_| "\"\"".into()),
    );
    let floating_script = init_script.clone();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            latch_window_action,
            latch_toggle_floating
        ])
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Latch IDE")
                .inner_size(1440.0, 900.0)
                .min_inner_size(960.0, 600.0)
                .decorations(false)
                .initialization_script(&init_script)
                .build()?;

            WebviewWindowBuilder::new(app, "floating", WebviewUrl::App("floating.html".into()))
                .title("Latch Quick Control")
                .inner_size(360.0, 400.0)
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .initialization_script(&floating_script)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("error while running Latch IDE desktop: {err}");
        });
}
