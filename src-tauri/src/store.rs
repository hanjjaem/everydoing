use std::collections::BTreeMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            scan_paths: vec![],
            extensions: ["hwp","hwpx","doc","docx","xls","xlsx","ppt","pptx","pdf"]
                .iter().map(|s| s.to_string()).collect(),
        }
    }
}

pub fn data_dir() -> PathBuf {
    let mut d = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    d.push("EveryDoing");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn config_path() -> PathBuf { let mut p = data_dir(); p.push("config.json"); p }
fn memos_path() -> PathBuf { let mut p = data_dir(); p.push("memos.json"); p }

pub fn load_config() -> Config {
    match std::fs::read_to_string(config_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => {
            let c = Config::default();
            let _ = save_config(c.clone());
            c
        }
    }
}

pub fn save_config(cfg: Config) -> Result<(), String> {
    let s = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(config_path(), s).map_err(|e| e.to_string())
}

pub fn load_memos() -> BTreeMap<String, String> {
    match std::fs::read_to_string(memos_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => BTreeMap::new(),
    }
}

pub fn save_memos(memos: &BTreeMap<String, String>) -> Result<(), String> {
    let s = serde_json::to_string_pretty(memos).map_err(|e| e.to_string())?;
    std::fs::write(memos_path(), s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
