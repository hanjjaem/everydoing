use std::collections::BTreeMap;
use serde::Serialize;
use chrono::{TimeZone, Local};

const FILETIME_POSIX_OFFSET: i64 = 11_644_473_600;

#[derive(Serialize, Clone)]
pub struct FileItem {
    pub name: String,
    pub path: String,
}

pub struct RawItem {
    pub name: String,
    pub path: String,
    /// Unix seconds (not FILETIME)
    pub date_modified: i64,
}

/// FILETIME(100ns ticks since 1601) → local date string. Kept for legacy/utility.
#[allow(dead_code)]
pub fn filetime_to_date(ft: i64) -> Option<String> {
    if ft <= 0 { return None; }
    let secs = ft / 10_000_000 - FILETIME_POSIX_OFFSET;
    unixtime_to_date(secs)
}

/// Unix seconds → local date string.
pub fn unixtime_to_date(secs: i64) -> Option<String> {
    if secs <= 0 { return None; }
    match Local.timestamp_opt(secs, 0).single() {
        Some(dt) => Some(dt.format("%Y-%m-%d").to_string()),
        None => None,
    }
}

pub fn group_by_date(results: Vec<RawItem>) -> BTreeMap<String, Vec<FileItem>> {
    let mut grouped: BTreeMap<String, Vec<FileItem>> = BTreeMap::new();
    for item in results {
        let date = unixtime_to_date(item.date_modified)
            .unwrap_or_else(|| "unknown".to_string());
        grouped.entry(date).or_default().push(FileItem { name: item.name, path: item.path });
    }
    grouped
}

pub fn drives_from_paths(scan_paths: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in scan_paths {
        let upper = p.to_uppercase();
        if upper.len() >= 2 && upper.as_bytes()[1] == b':' {
            let drive = upper[..2].to_string();
            if seen.insert(drive.clone()) {
                out.push(drive);
            }
        }
    }
    out
}

pub fn path_matches(full_path: &str, scan_paths: &[String]) -> bool {
    let lower_fp = full_path.to_lowercase().replace('/', "\\");
    for sp in scan_paths {
        let lower_sp = sp.to_lowercase().replace('/', "\\")
            .trim_end_matches('\\').to_string();
        if lower_fp.starts_with(&lower_sp) {
            let rest = &lower_fp[lower_sp.len()..];
            if rest.is_empty() || rest.starts_with('\\') {
                return true;
            }
        }
    }
    false
}

pub fn ext_matches(name: &str, extensions: &[String]) -> bool {
    let lower = name.to_lowercase();
    if let Some(pos) = lower.rfind('.') {
        let ext = &lower[pos + 1..];
        return extensions.iter().any(|e| e.to_lowercase() == ext);
    }
    false
}

#[derive(Serialize)]
pub struct ScanResult {
    pub files: BTreeMap<String, Vec<FileItem>>,
    pub skipped: Vec<String>,
}

pub fn scan(cfg: &crate::store::Config) -> ScanResult {
    use ntfs_reader::{file_info::FileInfo, mft::Mft, volume::Volume};

    let drives = drives_from_paths(&cfg.scan_paths);
    let mut raws: Vec<RawItem> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for drive in &drives {
        let volume_path = format!("\\\\.\\{}", drive);
        let volume = match Volume::new(&volume_path) {
            Ok(v) => v,
            Err(e) => { skipped.push(format!("{}: {}", drive, e)); continue; }
        };
        let mft = match Mft::new(volume) {
            Ok(m) => m,
            Err(e) => { skipped.push(format!("{}: {}", drive, e)); continue; }
        };
        let mut cache = ntfs_reader::file_info::VecCache::default();
        for file in mft.files() {
            let info = FileInfo::with_cache(&mft, &file, &mut cache);
            if info.is_directory { continue; }
            if !ext_matches(&info.name, &cfg.extensions) { continue; }
            let full_path = info.path.to_string_lossy().to_string();
            if !path_matches(&full_path, &cfg.scan_paths) { continue; }
            let unix_secs = info.modified
                .map(|odt| odt.unix_timestamp())
                .unwrap_or(0);
            raws.push(RawItem { name: info.name, path: full_path, date_modified: unix_secs });
        }
    }

    ScanResult { files: group_by_date(raws), skipped }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- filetime_to_date ---
    #[test]
    fn filetime_known_value() {
        // FILETIME 134256960000000000 → unix 1781222400 → 2026-06-12
        assert_eq!(filetime_to_date(134256960000000000), Some("2026-06-12".to_string()));
    }
    #[test]
    fn filetime_zero_is_none() {
        assert_eq!(filetime_to_date(0), None);
    }
    #[test]
    fn filetime_negative_is_none() {
        assert_eq!(filetime_to_date(-1), None);
    }

    // --- unixtime_to_date ---
    #[test]
    fn unixtime_known_value() {
        // 1781222400 = 2026-06-12 00:00:00 UTC → KST 2026-06-12
        assert_eq!(unixtime_to_date(1781222400), Some("2026-06-12".to_string()));
    }
    #[test]
    fn unixtime_zero_is_none() {
        assert_eq!(unixtime_to_date(0), None);
    }

    // --- group_by_date ---
    #[test]
    fn group_buckets_and_unknown() {
        // 1781222400 = unix secs for 2026-06-12 UTC
        let items = vec![
            RawItem { name: "a.hwp".into(), path: "C:\\x\\a.hwp".into(), date_modified: 1781222400 },
            RawItem { name: "c.pdf".into(), path: "C:\\c.pdf".into(), date_modified: 0 },
        ];
        let g = group_by_date(items);
        assert_eq!(g.get("2026-06-12").unwrap().len(), 1);
        assert_eq!(g.get("unknown").unwrap()[0].name, "c.pdf");
    }

    // --- drives_from_paths ---
    #[test]
    fn drives_extracts_unique_uppercase() {
        let paths = vec![
            "C:\\Users\\foo".to_string(),
            "c:\\Users\\bar".to_string(),
            "D:\\Work".to_string(),
        ];
        let mut drives = drives_from_paths(&paths);
        drives.sort();
        assert_eq!(drives, vec!["C:", "D:"]);
    }
    #[test]
    fn drives_empty_input() {
        assert_eq!(drives_from_paths(&[]), Vec::<String>::new());
    }
    #[test]
    fn drives_single() {
        assert_eq!(drives_from_paths(&["D:\\A\\B".to_string()]), vec!["D:"]);
    }

    // --- path_matches ---
    #[test]
    fn path_matches_exact_prefix_with_separator() {
        assert!(path_matches("C:\\Users\\foo\\file.hwp", &["C:\\Users\\foo".to_string()]));
    }
    #[test]
    fn path_matches_case_insensitive() {
        assert!(path_matches("c:\\users\\foo\\file.hwp", &["C:\\Users\\foo".to_string()]));
    }
    #[test]
    fn path_matches_no_false_positive_on_partial_dir() {
        assert!(!path_matches("C:\\Document\\file.hwp", &["C:\\Doc".to_string()]));
    }
    #[test]
    fn path_matches_none_if_no_scan_paths() {
        assert!(!path_matches("C:\\foo\\bar.hwp", &[]));
    }
    #[test]
    fn path_matches_multiple_candidates() {
        let scan = vec!["C:\\A".to_string(), "D:\\B".to_string()];
        assert!(path_matches("D:\\B\\file.pdf", &scan));
        assert!(!path_matches("E:\\C\\file.pdf", &scan));
    }

    // --- ext_matches ---
    #[test]
    fn ext_matches_basic() {
        assert!(ext_matches("report.hwp", &["hwp".to_string(), "pdf".to_string()]));
    }
    #[test]
    fn ext_matches_case_insensitive() {
        assert!(ext_matches("REPORT.HWP", &["hwp".to_string()]));
    }
    #[test]
    fn ext_no_match() {
        assert!(!ext_matches("image.png", &["hwp".to_string(), "pdf".to_string()]));
    }
    #[test]
    fn ext_no_extension() {
        assert!(!ext_matches("Makefile", &["hwp".to_string()]));
    }
    #[test]
    fn ext_empty_list() {
        assert!(!ext_matches("file.hwp", &[]));
    }
}
