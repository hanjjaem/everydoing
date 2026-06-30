#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;
mod indexer;
mod summarizer;

use std::collections::BTreeMap;
use indexer::ScanResult;
use tauri::{AppHandle, Emitter};

#[tauri::command]
fn load_cache() -> Option<ScanResult> {
    let cfg = store::load_config();
    store::load_index(&cfg)
}

fn is_drive_root(path: &str) -> bool {
    let p = path.trim_end_matches('\\');
    p.len() == 2 && p.as_bytes()[1] == b':'
}

#[tauri::command]
async fn start_scan(app: AppHandle) {
    let cfg = store::load_config();
    let use_mft = cfg.scan_paths.iter().any(|p| is_drive_root(p));
    let result = tauri::async_runtime::spawn_blocking({
        let app2 = app.clone();
        move || {
            if use_mft {
                indexer::scan(&cfg, &app2)
            } else {
                indexer::scan_walk_with_progress(&cfg, &app2)
            }
        }
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
fn save_config(
    scan_paths: Vec<String>,
    extensions: Vec<String>,
    openai_api_key: Option<String>,
    ai_summary_enabled: Option<bool>,
) -> Result<(), String> {
    let (paths, exts) = store::validate_config(scan_paths, extensions)?;
    let mut cfg = store::load_config();
    cfg.scan_paths = paths;
    cfg.extensions = exts;
    if let Some(key) = openai_api_key {
        cfg.openai_api_key = key;
    }
    if let Some(enabled) = ai_summary_enabled {
        cfg.ai_summary_enabled = enabled;
    }
    store::save_config(cfg)
}

#[tauri::command]
fn get_summaries() -> BTreeMap<String, String> {
    store::load_summaries()
}

#[tauri::command]
fn save_summary(date: String, text: String) -> Result<(), String> {
    let mut s = store::load_summaries();
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() { s.remove(&date); } else { s.insert(date, trimmed); }
    store::save_summaries(&s)
}

#[tauri::command]
async fn generate_summaries() -> Result<BTreeMap<String, String>, String> {
    let cfg = store::load_config();
    // 전송 게이트: 기능이 꺼져 있으면 어떤 데이터도 외부로 내보내지 않는다.
    if !cfg.ai_summary_enabled {
        return Err("AI 요약이 꺼져 있습니다. 설정에서 켜야 사용할 수 있습니다".into());
    }
    if cfg.openai_api_key.is_empty() {
        return Err("OpenAI API 키가 설정되지 않았습니다".into());
    }
    let scan_result = match store::load_index(&cfg) {
        Some(r) => r,
        None => return Err("스캔 캐시가 없습니다".into()),
    };
    let files_by_date: BTreeMap<String, Vec<String>> = scan_result.files
        .iter()
        .map(|(date, files)| (date.clone(), files.iter().map(|f| f.name.clone()).collect()))
        .collect();
    let summaries = summarizer::summarize_dates(&files_by_date, &cfg.openai_api_key).await;
    let _ = store::save_summaries(&summaries);
    Ok(summaries)
}

#[tauri::command]
fn get_monthly_summaries() -> BTreeMap<String, String> {
    store::load_monthly_summaries()
}

#[tauri::command]
async fn generate_monthly_summary(year: i32, month: u32) -> Result<String, String> {
    let cfg = store::load_config();
    // 전송 게이트: 기능이 꺼져 있으면 어떤 데이터도 외부로 내보내지 않는다.
    if !cfg.ai_summary_enabled {
        return Err("AI 요약이 꺼져 있습니다. 설정에서 켜야 사용할 수 있습니다".into());
    }
    if cfg.openai_api_key.is_empty() {
        return Err("OpenAI API 키가 설정되지 않았습니다".into());
    }
    let scan_result = match store::load_index(&cfg) {
        Some(r) => r,
        None => return Err("스캔 캐시가 없습니다".into()),
    };
    let prefix = format!("{:04}-{:02}", year, month);
    let files_by_date: BTreeMap<String, Vec<String>> = scan_result.files
        .iter()
        .filter(|(date, _)| date.starts_with(&prefix))
        .map(|(date, files)| (date.clone(), files.iter().map(|f| f.name.clone()).collect()))
        .collect();
    let ym_label = format!("{}년 {}월", year, month);
    let text = summarizer::summarize_month(&files_by_date, &ym_label, &cfg.openai_api_key).await?;
    let mut all = store::load_monthly_summaries();
    all.insert(prefix, text.clone());
    let _ = store::save_monthly_summaries(&all);
    Ok(text)
}

#[tauri::command]
fn prune_deleted() -> Option<ScanResult> {
    let cfg = store::load_config();
    let mut result = store::load_index(&cfg)?;
    let mut changed = false;
    for files in result.files.values_mut() {
        let before = files.len();
        files.retain(|f| std::path::Path::new(&f.path).exists());
        if files.len() != before { changed = true; }
    }
    result.files.retain(|_, files| !files.is_empty());
    if changed { let _ = store::save_index(&cfg, &result); }
    Some(result)
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
            load_cache, start_scan, prune_deleted,
            open_path, pick_folder,
            get_config, save_config,
            get_memos, save_memo, copy_text,
            get_summaries, generate_summaries, save_summary,
            get_monthly_summaries, generate_monthly_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
