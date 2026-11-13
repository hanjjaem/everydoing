use std::collections::BTreeMap;
use std::path::PathBuf;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};
use crate::indexer::ScanResult;

pub fn normalize_extension(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_string();
    if let Some(stripped) = s.strip_prefix('.') { s = stripped.to_string(); }
    let s = s.trim().to_lowercase();
    if s.is_empty() { None } else { Some(s) }
}

pub fn normalize_extensions(raws: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for r in raws {
        if let Some(e) = normalize_extension(r) {
            if !out.contains(&e) { out.push(e); }
        }
    }
    out
}

pub fn set_memo(memos: &BTreeMap<String, String>, date: &str, text: &str) -> BTreeMap<String, String> {
    let mut out = memos.clone();
    let trimmed = text.trim();
    if trimmed.is_empty() { out.remove(date); } else { out.insert(date.to_string(), trimmed.to_string()); }
    out
}

pub fn validate_config(
    scan_paths: Vec<String>,
    extensions: Vec<String>,
) -> Result<(Vec<String>, Vec<String>), String> {
    if scan_paths.is_empty() { return Err("스캔 폴더를 최소 1개 지정하세요".into()); }
    let exts = normalize_extensions(&extensions);
    if exts.is_empty() { return Err("파일 유형을 최소 1개 선택하세요".into()); }
    Ok((scan_paths, exts))
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub scan_paths: Vec<String>,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub openai_api_key: String,
    /// AI 요약 기능 사용 여부. 기본값 false(꺼짐).
    /// 이 값이 false면 파일명이 외부(OpenAI)로 전송되지 않는다.
    #[serde(default)]
    pub ai_summary_enabled: bool,
    /// 시작할 때 새 버전 확인 여부. 기본값 false(꺼짐).
    /// 이 값이 false면 사용자가 '지금 확인'을 누르기 전에는 GitHub에 접속하지 않는다.
    #[serde(default)]
    pub update_check_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scan_paths: vec![],
            extensions: ["hwp","hwpx","doc","docx","xls","xlsx","ppt","pptx","pdf"]
                .iter().map(|s| s.to_string()).collect(),
            openai_api_key: String::new(),
            ai_summary_enabled: false,
            update_check_enabled: false,
        }
    }
}

pub fn data_dir() -> PathBuf {
    // Explicitly isolated previews never touch the installed app's data.
    #[cfg(debug_assertions)]
    if let Some(path) = preview_data_dir(std::env::args_os()) {
        let _ = std::fs::create_dir_all(&path);
        return path;
    }
    let mut d = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    d.push("EveryDoing");
    let _ = std::fs::create_dir_all(&d);
    d
}

#[cfg(debug_assertions)]
fn preview_data_dir(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<PathBuf> {
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--preview-data-dir" {
            if let Some(path) = args.next().map(PathBuf::from).filter(|path| path.is_absolute()) {
                return Some(path);
            }
            eprintln!("--preview-data-dir requires an absolute directory");
            std::process::exit(2);
        }
    }
    None
}

fn config_path() -> PathBuf { let mut p = data_dir(); p.push("config.json"); p }
fn memos_path() -> PathBuf { let mut p = data_dir(); p.push("memos.json"); p }
fn index_path() -> PathBuf { let mut p = data_dir(); p.push("index.json"); p }
fn summaries_path() -> PathBuf { let mut p = data_dir(); p.push("summaries.json"); p }

fn load_map(path: &std::path::Path) -> Result<BTreeMap<String, String>, String> {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("{} 파일을 읽을 수 없습니다: {}", path.display(), e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(format!("{} 파일을 읽을 수 없습니다: {}", path.display(), e)),
    }
}

// Write next to the original and replace it only after the complete data is on disk.
fn write_json<T: Serialize>(path: &std::path::Path, value: &T) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let temp = path.with_extension(format!("tmp-{}-{}", std::process::id(), id));
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temp, path)
    })();
    if result.is_err() { let _ = std::fs::remove_file(&temp); }
    result.map_err(|e| format!("{} 파일을 저장할 수 없습니다: {}", path.display(), e))
}

pub fn load_summaries() -> Result<BTreeMap<String, String>, String> {
    load_map(&summaries_path())
}

pub fn save_summaries(summaries: &BTreeMap<String, String>) -> Result<(), String> {
    write_json(&summaries_path(), summaries)
}

fn monthly_summaries_path() -> PathBuf { let mut p = data_dir(); p.push("monthly_summaries.json"); p }

/// 월별 총평(키 "YYYY-MM" → 줄글 텍스트) 로드.
pub fn load_monthly_summaries() -> Result<BTreeMap<String, String>, String> {
    load_map(&monthly_summaries_path())
}

pub fn save_monthly_summaries(m: &BTreeMap<String, String>) -> Result<(), String> {
    write_json(&monthly_summaries_path(), m)
}

#[derive(Serialize, Deserialize)]
struct IndexCache {
    scan_paths: Vec<String>,
    extensions: Vec<String>,
    #[serde(flatten)]
    result: ScanResult,
}

pub fn save_index(cfg: &Config, result: &ScanResult) -> Result<(), String> {
    save_index_at(&index_path(), cfg, result)
}

fn save_index_at(path: &std::path::Path, cfg: &Config, result: &ScanResult) -> Result<(), String> {
    let cache = IndexCache {
        scan_paths: cfg.scan_paths.clone(),
        extensions: cfg.extensions.clone(),
        result: result.clone(),
    };
    write_json(path, &cache)
}

pub fn load_index(cfg: &Config) -> Option<ScanResult> {
    load_index_at(&index_path(), cfg)
}

fn load_index_at(path: &std::path::Path, cfg: &Config) -> Option<ScanResult> {
    let s = std::fs::read_to_string(path).ok()?;
    let cache: IndexCache = serde_json::from_str(&s).ok()?;
    if cache.scan_paths != cfg.scan_paths || cache.extensions != cfg.extensions {
        return None;
    }
    Some(cache.result)
}

pub fn load_config() -> Result<Config, String> {
    match std::fs::read_to_string(config_path()) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("설정 파일을 읽을 수 없습니다: {}", e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("설정 파일을 읽을 수 없습니다: {}", e)),
    }
}

pub fn save_config(cfg: Config) -> Result<(), String> {
    write_json(&config_path(), &cfg)
}

pub fn load_memos() -> Result<BTreeMap<String, String>, String> {
    load_map(&memos_path())
}

pub fn save_memos(memos: &BTreeMap<String, String>) -> Result<(), String> {
    write_json(&memos_path(), memos)
}

// 아래 업데이트 확인 항목은 데스크톱 앱(main.rs)만 쓴다. 브라우저 개발 모드(dev_web)는 업데이트를 지원하지 않는다.

/// 마지막 업데이트 확인 결과.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone)]
pub struct UpdateCache {
    pub latest_version: String,
    /// 확인한 시각(Unix 초).
    pub checked_at: i64,
}

#[allow(dead_code)]
fn update_cache_path() -> PathBuf { let mut p = data_dir(); p.push("update.json"); p }

/// 저장된 확인 결과. 없거나 깨졌으면 None(다시 확인하면 된다).
#[allow(dead_code)]
pub fn load_update_cache() -> Option<UpdateCache> {
    let s = std::fs::read_to_string(update_cache_path()).ok()?;
    serde_json::from_str(&s).ok()
}

#[allow(dead_code)]
pub fn save_update_cache(cache: &UpdateCache) -> Result<(), String> {
    write_json(&update_cache_path(), cache)
}

const UPDATE_CHECK_INTERVAL_SECS: i64 = 20 * 3600;

/// 다시 확인할 때가 됐는지. 기록이 없거나, 시계가 거꾸로 갔거나, 20시간이 지났으면 true.
#[allow(dead_code)]
pub fn update_check_due(cache: Option<&UpdateCache>, now: i64) -> bool {
    match cache {
        None => true,
        Some(c) => c.checked_at > now || now - c.checked_at >= UPDATE_CHECK_INTERVAL_SECS,
    }
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim();
    let v = v.strip_prefix('v').unwrap_or(v);
    let core = v.split('-').next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let (major, minor, patch) = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() { return None; }
    Some((major, minor, patch))
}

/// latest가 current보다 새 버전인지("x.y.z" 숫자 비교). 읽을 수 없는 버전이면 false.
#[allow(dead_code)]
pub fn is_newer_version(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(debug_assertions)]
    #[test]
    fn preview_data_dir_uses_only_explicit_absolute_path() {
        let path = std::env::temp_dir().join("everydoing-preview-test");
        let args = vec!["everydoing".into(), "--preview-data-dir".into(), path.clone().into_os_string()];
        assert_eq!(super::preview_data_dir(args), Some(path));
        assert_eq!(super::preview_data_dir([std::ffi::OsString::from("everydoing")]), None);
    }
    use super::*;

    #[test]
    fn corrupt_memo_data_is_reported_without_overwriting_it() {
        let path = std::env::temp_dir().join(format!("everydoing-corrupt-{}.json", std::process::id()));
        std::fs::write(&path, "{broken").unwrap();
        assert!(load_map(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{broken");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn ext_strips_dot_and_lowercases() {
        assert_eq!(normalize_extension(" .HWP "), Some("hwp".to_string()));
    }
    #[test]
    fn ext_empty_is_none() {
        assert_eq!(normalize_extension("  "), None);
        assert_eq!(normalize_extension("."), None);
    }
    #[test]
    fn exts_dedupe_keep_order_drop_none() {
        let input = vec!["PDF".into(), ".pdf".into(), "hwp".into(), "".into(), "docx".into()];
        assert_eq!(normalize_extensions(&input), vec!["pdf", "hwp", "docx"]);
    }
    #[test]
    fn set_memo_adds_and_strips() {
        let m = BTreeMap::new();
        let out = set_memo(&m, "2026-06-25", "  보고서  ");
        assert_eq!(out.get("2026-06-25"), Some(&"보고서".to_string()));
    }
    #[test]
    fn set_memo_blank_deletes() {
        let mut m = BTreeMap::new();
        m.insert("2026-06-25".to_string(), "x".to_string());
        let out = set_memo(&m, "2026-06-25", "   ");
        assert!(out.get("2026-06-25").is_none());
    }
    #[test]
    fn validate_config_rejects_empty_paths() {
        let r = validate_config(vec![], vec!["hwp".into()]);
        assert!(r.is_err());
    }
    #[test]
    fn validate_config_rejects_empty_exts_after_norm() {
        let r = validate_config(vec!["C:\\A".into()], vec![".".into(), "".into()]);
        assert!(r.is_err());
    }
    #[test]
    fn validate_config_ok_normalizes_exts() {
        let (paths, exts) = validate_config(vec!["C:\\A".into()], vec!["PDF".into(), ".pdf".into()]).unwrap();
        assert_eq!(paths, vec!["C:\\A"]);
        assert_eq!(exts, vec!["pdf"]);
    }

    #[test]
    fn index_roundtrip() {
        use std::collections::BTreeMap;
        use crate::indexer::{ScanResult, FileItem};
        let cfg = Config {
            scan_paths: vec!["D:\\test".to_string()],
            extensions: vec!["pdf".to_string()],
            ..Default::default()
        };
        let mut files = BTreeMap::new();
        files.insert("2026-06-29".to_string(), vec![FileItem {
            name: "a.pdf".to_string(),
            path: "D:\\test\\a.pdf".to_string(),
            modified: 1781222400,
        }]);
        let result = ScanResult { files, scanned_at: 1781222500, ..Default::default() };
        let path = std::env::temp_dir().join(format!("everydoing-index-roundtrip-{}.json", std::process::id()));
        save_index_at(&path, &cfg, &result).unwrap();
        let loaded = load_index_at(&path, &cfg).unwrap();
        assert_eq!(loaded.files["2026-06-29"][0].name, "a.pdf");
        assert_eq!(loaded.files["2026-06-29"][0].modified, 1781222400);
        assert_eq!(loaded.scanned_at, 1781222500);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn index_from_older_version_still_loads() {
        // 수정 시각·스캔 시각이 없던 0.1.0 색인도 그대로 읽혀야 한다.
        let cfg = Config {
            scan_paths: vec!["D:\\test".to_string()],
            extensions: vec!["pdf".to_string()],
            ..Default::default()
        };
        let old = r#"{"scan_paths":["D:\\test"],"extensions":["pdf"],
            "files":{"2026-06-29":[{"name":"a.pdf","path":"D:\\test\\a.pdf"}]},"skipped":[]}"#;
        let path = std::env::temp_dir().join(format!("everydoing-index-old-{}.json", std::process::id()));
        std::fs::write(&path, old).unwrap();
        let loaded = load_index_at(&path, &cfg).unwrap();
        assert_eq!(loaded.files["2026-06-29"][0].modified, 0);
        assert_eq!(loaded.scanned_at, 0);
        assert!(!loaded.root_failed);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn update_check_due_boundaries() {
        let now = 1_800_000_000;
        let at = |checked_at| UpdateCache { latest_version: "0.3.0".into(), checked_at };
        assert!(update_check_due(None, now), "기록이 없으면 확인한다");
        assert!(!update_check_due(Some(&at(now)), now));
        assert!(!update_check_due(Some(&at(now - 20 * 3600 + 1)), now), "20시간이 안 됐으면 건너뛴다");
        assert!(update_check_due(Some(&at(now - 20 * 3600)), now), "딱 20시간이면 확인한다");
        assert!(update_check_due(Some(&at(now + 1)), now), "시계가 거꾸로 갔으면 확인한다");
    }

    #[test]
    fn is_newer_version_compares_numbers() {
        assert!(is_newer_version("0.3.1", "0.3.0"));
        assert!(is_newer_version("0.10.0", "0.9.9"), "문자열이 아니라 숫자로 비교한다");
        assert!(is_newer_version("1.0.0", "0.99.99"));
        assert!(!is_newer_version("0.3.0", "0.3.0"));
        assert!(!is_newer_version("0.2.9", "0.3.0"));
        assert!(is_newer_version("v0.4.0", "0.3.0"), "앞의 v는 허용한다");
        assert!(!is_newer_version("0.3.0-beta.1", "0.3.0"), "프리릴리스 꼬리는 무시한다");
        assert!(is_newer_version("0.3.1-rc", "0.3.0"));
        assert!(!is_newer_version("abc", "0.3.0"), "읽을 수 없으면 false");
        assert!(!is_newer_version("0.4", "0.3.0"));
        assert!(!is_newer_version("0.4.0.1", "0.3.0"));
        assert!(!is_newer_version("0.4.0", ""));
    }

    #[test]
    fn config_without_update_field_reads_as_off() {
        let old = r#"{"scan_paths":["D:\\a"],"extensions":["pdf"],"openai_api_key":"","ai_summary_enabled":false}"#;
        let cfg: Config = serde_json::from_str(old).unwrap();
        assert!(!cfg.update_check_enabled);
        assert!(!Config::default().update_check_enabled);
    }

    #[test]
    fn index_invalidated_when_paths_differ() {
        use crate::indexer::ScanResult;
        let cfg_a = Config {
            scan_paths: vec!["D:\\a".to_string()],
            extensions: vec!["pdf".to_string()],
            ..Default::default()
        };
        let cfg_b = Config {
            scan_paths: vec!["D:\\b".to_string()],
            extensions: vec!["pdf".to_string()],
            ..Default::default()
        };
        let result = ScanResult::default();
        let path = std::env::temp_dir().join(format!("everydoing-index-invalidated-{}.json", std::process::id()));
        save_index_at(&path, &cfg_a, &result).unwrap();
        assert!(load_index_at(&path, &cfg_b).is_none());
        std::fs::remove_file(path).unwrap();
    }
}
