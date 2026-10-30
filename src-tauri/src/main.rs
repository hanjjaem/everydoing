#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;
mod indexer;
mod summarizer;

use std::collections::BTreeMap;
use indexer::ScanResult;
use tauri::{AppHandle, Emitter};
use std::sync::Mutex;

static MEMO_WRITE: Mutex<()> = Mutex::new(());
static SUMMARY_WRITE: Mutex<()> = Mutex::new(());
static MONTHLY_WRITE: Mutex<()> = Mutex::new(());

#[tauri::command]
fn load_cache() -> Option<ScanResult> {
    let cfg = store::load_config().ok()?;
    store::load_index(&cfg)
}

fn is_drive_root(path: &str) -> bool {
    let p = path.trim_end_matches('\\');
    p.len() == 2 && p.as_bytes()[1] == b':'
}

#[tauri::command]
async fn start_scan(app: AppHandle) -> Result<(), String> {
    let cfg = store::load_config()?;
    let scanned_cfg = cfg.clone();
    let mut result = tauri::async_runtime::spawn_blocking({
        let app2 = app.clone();
        move || {
            let roots: Vec<_> = cfg.scan_paths.iter().filter(|p| is_drive_root(p)).cloned().collect();
            let folders: Vec<_> = cfg.scan_paths.iter()
                .filter(|p| !is_drive_root(p) && !indexer::path_matches(p, &roots))
                .cloned().collect();
            let mut combined = ScanResult { files: BTreeMap::new(), skipped: Vec::new() };
            if !roots.is_empty() {
                let mut root_cfg = cfg.clone();
                root_cfg.scan_paths = roots;
                let result = indexer::scan(&root_cfg, &app2);
                combined.files = result.files;
                combined.skipped = result.skipped;
            }
            if !folders.is_empty() {
                let mut folder_cfg = cfg;
                folder_cfg.scan_paths = folders;
                let result = indexer::scan_walk_with_progress(&folder_cfg, &app2);
                for (date, files) in result.files { combined.files.entry(date).or_default().extend(files); }
                combined.skipped.extend(result.skipped);
            }
            combined
        }
    }).await.map_err(|e| format!("스캔 작업 오류: {}", e))?;
    let cfg = store::load_config()?;
    if cfg.scan_paths != scanned_cfg.scan_paths || cfg.extensions != scanned_cfg.extensions {
        return Err("스캔 중 설정이 변경되었습니다. 다시 스캔해 주세요.".into());
    }
    if result.skipped.is_empty() {
        if let Err(e) = store::save_index(&scanned_cfg, &result) {
            result.skipped.push(format!("캐시 저장 실패: {}", e));
        }
    }
    app.emit("scan-done", result).map_err(|e| e.to_string())?;
    Ok(())
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
    store::load_config()
}

#[tauri::command]
fn save_config(
    scan_paths: Vec<String>,
    extensions: Vec<String>,
    openai_api_key: Option<String>,
    ai_summary_enabled: Option<bool>,
) -> Result<(), String> {
    let (paths, exts) = store::validate_config(scan_paths, extensions)?;
    let mut cfg = store::load_config()?;
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
fn get_summaries() -> Result<BTreeMap<String, String>, String> {
    store::load_summaries()
}

#[tauri::command]
fn save_summary(date: String, text: String) -> Result<(), String> {
    let _guard = SUMMARY_WRITE.lock().map_err(|e| e.to_string())?;
    let mut s = store::load_summaries()?;
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() { s.remove(&date); } else { s.insert(date, trimmed); }
    store::save_summaries(&s)
}

#[tauri::command]
async fn generate_summaries() -> Result<BTreeMap<String, String>, String> {
    let cfg = store::load_config()?;
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
    let generated = summarizer::summarize_dates(&files_by_date, &cfg.openai_api_key).await?;
    let _guard = SUMMARY_WRITE.lock().map_err(|e| e.to_string())?;
    let mut summaries = store::load_summaries()?;
    summaries.extend(generated.into_iter().filter(|(date, _)| files_by_date.contains_key(date)));
    store::save_summaries(&summaries)?;
    Ok(summaries)
}

#[tauri::command]
fn get_monthly_summaries() -> Result<BTreeMap<String, String>, String> {
    store::load_monthly_summaries()
}

#[tauri::command]
async fn generate_monthly_summary(year: i32, month: u32) -> Result<String, String> {
    let cfg = store::load_config()?;
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
    let _guard = MONTHLY_WRITE.lock().map_err(|e| e.to_string())?;
    let mut all = store::load_monthly_summaries()?;
    all.insert(prefix, text.clone());
    store::save_monthly_summaries(&all)?;
    Ok(text)
}

#[tauri::command]
fn prune_deleted() -> Option<ScanResult> {
    let cfg = store::load_config().ok()?;
    let mut result = store::load_index(&cfg)?;
    // A disconnected or inaccessible scan root must not be treated as deleted files.
    if cfg.scan_paths.iter().any(|path| std::fs::metadata(path).is_err()) {
        return Some(result);
    }
    let mut changed = false;
    for files in result.files.values_mut() {
        let before = files.len();
        files.retain(|f| match std::fs::metadata(&f.path) {
            Ok(_) => true,
            Err(e) => e.kind() != std::io::ErrorKind::NotFound,
        });
        if files.len() != before { changed = true; }
    }
    result.files.retain(|_, files| !files.is_empty());
    if changed { let _ = store::save_index(&cfg, &result); }
    Some(result)
}

#[tauri::command]
fn get_memos() -> Result<BTreeMap<String, String>, String> {
    store::load_memos()
}

#[tauri::command]
fn save_memo(date: String, text: String) -> Result<(), String> {
    if date.is_empty() { return Err("date 필요".into()); }
    let _guard = MEMO_WRITE.lock().map_err(|e| e.to_string())?;
    let memos = store::set_memo(&store::load_memos()?, &date, &text);
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
