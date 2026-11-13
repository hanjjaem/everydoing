#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;
mod indexer;
mod summarizer;

use std::collections::BTreeMap;
use indexer::ScanResult;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;
use std::sync::Mutex;
use std::time::Duration;

static MEMO_WRITE: Mutex<()> = Mutex::new(());
static SUMMARY_WRITE: Mutex<()> = Mutex::new(());
static MONTHLY_WRITE: Mutex<()> = Mutex::new(());

#[tauri::command]
fn load_cache() -> Option<ScanResult> {
    let cfg = store::load_config().ok()?;
    store::load_index(&cfg)
}

#[tauri::command]
async fn start_scan(app: AppHandle) -> Result<(), String> {
    let cfg = store::load_config()?;
    let scanned_cfg = cfg.clone();
    let mut result = tauri::async_runtime::spawn_blocking({
        let app2 = app.clone();
        move || indexer::scan_all(&cfg, Some(&app2))
    }).await.map_err(|e| format!("스캔 작업 오류: {}", e))?;
    let cfg = store::load_config()?;
    if cfg.scan_paths != scanned_cfg.scan_paths || cfg.extensions != scanned_cfg.extensions {
        return Err("스캔 중 설정이 변경되었습니다. 다시 스캔해 주세요.".into());
    }
    // 하위 폴더·파일 일부를 못 읽은 것은 저장한다(매번 전체 스캔을 반복하지 않게).
    // 스캔 폴더 자체를 못 읽었으면 결과가 불완전하므로 이전 색인을 지킨다.
    if !result.root_failed {
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
    update_check_enabled: Option<bool>,
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
    if let Some(enabled) = update_check_enabled {
        cfg.update_check_enabled = enabled;
    }
    store::save_config(cfg)
}

#[derive(serde::Serialize)]
struct UpdateStatus {
    current_version: String,
    latest_version: Option<String>,
    checked_at: i64,
    update_available: bool,
    check_enabled: bool,
}

fn update_status(app: &AppHandle, cache: Option<&store::UpdateCache>, check_enabled: bool) -> UpdateStatus {
    let current_version = app.package_info().version.to_string();
    let latest_version = cache.map(|c| c.latest_version.clone());
    let update_available = latest_version.as_deref()
        .map(|latest| store::is_newer_version(latest, &current_version))
        .unwrap_or(false);
    UpdateStatus {
        current_version,
        latest_version,
        checked_at: cache.map(|c| c.checked_at).unwrap_or(0),
        update_available,
        check_enabled,
    }
}

async fn find_update(app: &AppHandle) -> Result<Option<tauri_plugin_updater::Update>, String> {
    let updater = app.updater_builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("업데이트 확인을 준비하지 못했습니다: {}", e))?;
    updater.check().await.map_err(|e| format!("새 버전을 확인하지 못했습니다: {}", e))
}

/// 저장된 확인 결과와 설정만 읽는다. 네트워크에 접속하지 않는다.
#[tauri::command]
fn get_update_status(app: AppHandle) -> Result<UpdateStatus, String> {
    let cfg = store::load_config()?;
    Ok(update_status(&app, store::load_update_cache().as_ref(), cfg.update_check_enabled))
}

/// force=false(시작할 때): 설정이 켜져 있고 확인할 때가 됐을 때만 접속한다.
/// force=true('지금 확인' 버튼): 설정과 상관없이 접속한다.
#[tauri::command]
async fn check_update(app: AppHandle, force: bool) -> Result<UpdateStatus, String> {
    let cfg = store::load_config()?;
    let cache = store::load_update_cache();
    let now = chrono::Utc::now().timestamp();
    if !force && (!cfg.update_check_enabled || !store::update_check_due(cache.as_ref(), now)) {
        return Ok(update_status(&app, cache.as_ref(), cfg.update_check_enabled));
    }
    let latest_version = match find_update(&app).await? {
        Some(update) => update.version,
        None => app.package_info().version.to_string(),
    };
    let cache = store::UpdateCache { latest_version, checked_at: now };
    store::save_update_cache(&cache)?;
    Ok(update_status(&app, Some(&cache), cfg.update_check_enabled))
}

/// 사용자가 업데이트 버튼을 눌렀을 때만 불린다.
/// Windows에서는 설치 프로그램을 실행하면서 앱이 끝나므로, 성공하면 반환되지 않는다.
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<String, String> {
    let Some(update) = find_update(&app).await? else {
        // 방금 확인한 결과를 남겨, 화면을 다시 그리면 알림 버튼이 사라지게 한다(캐시라 저장 실패는 무시).
        let _ = store::save_update_cache(&store::UpdateCache {
            latest_version: app.package_info().version.to_string(),
            checked_at: chrono::Utc::now().timestamp(),
        });
        return Ok("이미 최신 버전입니다".into());
    };
    update.download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| format!("업데이트를 설치하지 못했습니다: {}", e))?;
    Ok("업데이트를 설치했습니다. 앱을 다시 시작해 주세요.".into())
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

/// 보고 있는 달(YYYY-MM)의 파일 중 지워진 것을 색인에서 빼고, 뺀 경로를 돌려준다.
/// 창에 포커스가 올 때마다 불리므로 전체가 아니라 한 달만, 메인 스레드 밖에서 확인한다.
#[tauri::command]
async fn prune_deleted(month: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<String>, String> {
        let cfg = store::load_config()?;
        let Some(mut result) = store::load_index(&cfg) else { return Ok(Vec::new()); };
        // A disconnected or inaccessible scan root must not be treated as deleted files.
        if cfg.scan_paths.iter().any(|path| std::fs::metadata(path).is_err()) {
            return Ok(Vec::new());
        }
        let removed = indexer::prune_month(&mut result, &month, |path| {
            matches!(std::fs::metadata(path), Err(e) if e.kind() == std::io::ErrorKind::NotFound)
        });
        if !removed.is_empty() { store::save_index(&cfg, &result)?; }
        Ok(removed)
    }).await.map_err(|e| format!("삭제 확인 작업 오류: {}", e))?
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            load_cache, start_scan, prune_deleted,
            open_path, pick_folder,
            get_config, save_config,
            get_memos, save_memo, copy_text,
            get_summaries, generate_summaries, save_summary,
            get_monthly_summaries, generate_monthly_summary,
            get_update_status, check_update, install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
