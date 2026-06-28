#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;
mod indexer;

use std::collections::BTreeMap;
use indexer::ScanResult;
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn load_cache() -> Option<ScanResult> {
    let cfg = store::load_config();
    store::load_index(&cfg)
}

#[tauri::command]
async fn start_scan(app: AppHandle) {
    let cfg = store::load_config();
    let result = tauri::async_runtime::spawn_blocking({
        let app2 = app.clone();
        move || indexer::scan(&cfg, &app2)
    }).await.unwrap_or_else(|_| ScanResult {
        files: BTreeMap::new(),
        skipped: vec!["스캔 스레드 오류".to_string()],
    });
    let cfg = store::load_config();
    let _ = store::save_index(&cfg, &result);
    let _ = app.emit("scan-done", result);
}

#[tauri::command]
fn open_path(path: String, mode: String) -> Result<(), String> {
    if !std::path::Path::new(&path).exists() {
        return Err("파일을 찾을 수 없습니다".into());
    }
    if mode == "folder" {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    } else {
        opener::open(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn pick_folder() -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new().pick_folder();
    Ok(picked.map(|p| p.to_string_lossy().to_string()))
}

#[tauri::command]
fn get_config() -> Result<store::Config, String> {
    Ok(store::load_config())
}

#[tauri::command]
fn save_config(scan_paths: Vec<String>, extensions: Vec<String>) -> Result<(), String> {
    let (paths, exts) = store::validate_config(scan_paths, extensions)?;
    let mut cfg = store::load_config();
    cfg.scan_paths = paths;
    cfg.extensions = exts;
    store::save_config(cfg)
}

#[tauri::command]
fn get_memos() -> Result<BTreeMap<String, String>, String> {
    Ok(store::load_memos())
}

#[tauri::command]
fn save_memo(date: String, text: String) -> Result<(), String> {
    if date.is_empty() { return Err("date 필요".into()); }
    let memos = store::set_memo(&store::load_memos(), &date, &text);
    store::save_memos(&memos)
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_cache, start_scan,
            open_path, pick_folder,
            get_config, save_config,
            get_memos, save_memo, copy_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
