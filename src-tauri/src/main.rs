#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;
mod everything;

use std::collections::BTreeMap;
use everything::FileItem;

#[tauri::command]
fn list_files() -> Result<BTreeMap<String, Vec<FileItem>>, String> {
    let cfg = store::load_config();
    everything::fetch_files(&cfg)
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
            list_files, open_path, pick_folder,
            get_config, save_config, get_memos, save_memo, copy_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
