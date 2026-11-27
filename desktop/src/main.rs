#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]
//! Shell Desktop Native Tauri v2 untuk Latch IDE.
//! Menyalakan daemon Latch di latar belakang dan menyuntikkan bearer token lokal
//! langsung ke WebView agar pengguna bisa langsung memakai aplikasi tanpa setup.

use std::time::Duration;

use latch_core::config::{load_latch_config, RuntimePaths};
use latch_daemon::http_server::ensure_or_load_token;
use tauri::{WebviewUrl, WebviewWindowBuilder};

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
        "window.__LATCH_API_BASE__ = {}; window.__LATCH_TOKEN__ = {};",
        serde_json::to_string(&api_base).unwrap_or_else(|_| "\"http://127.0.0.1:7878\"".into()),
        serde_json::to_string(&token).unwrap_or_else(|_| "\"\"".into()),
    );

    tauri::Builder::default()
        .setup(move |app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Latch IDE")
                .inner_size(1320.0, 840.0)
                .min_inner_size(960.0, 620.0)
                .initialization_script(&init_script)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| {
            eprintln!("error while running Latch IDE desktop: {err}");
        });
}
