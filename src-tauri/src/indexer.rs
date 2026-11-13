use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use chrono::{TimeZone, Local};
use tauri::Emitter;

const FILETIME_POSIX_OFFSET: i64 = 11_644_473_600;

#[derive(Serialize, Deserialize, Clone)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    /// 수정 시각(Unix 초). 이 필드가 없던 옛 색인은 0으로 읽힌다.
    #[serde(default)]
    pub modified: i64,
}

pub struct RawItem {
    pub name: String,
    pub path: String,
    /// Unix seconds (not FILETIME)
    pub date_modified: i64,
}

#[derive(Serialize, Clone)]
pub struct ScanProgress {
    pub drive: String,
    pub scanned: u64,
    pub total: u64,
    pub matched: u64,
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
        grouped.entry(date).or_default().push(FileItem {
            name: item.name, path: item.path, modified: item.date_modified,
        });
    }
    // 하루 안에서는 최근에 고친 파일이 먼저 오게 한다.
    for files in grouped.values_mut() {
        files.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));
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

/// 드라이브 맨 위에 있는 시스템 폴더. 드라이브 전체를 스캔할 때만 제외한다.
const SYSTEM_TOP_DIRS: [&str; 6] = [
    "$recycle.bin", "system volume information",
    "windows", "program files", "program files (x86)", "programdata",
];

/// 업무 문서가 아닌 것인지 판단한다: 휴지통·시스템 폴더, 점(.)으로 시작하는 폴더,
/// node_modules, Office가 편집 중에 만드는 임시 파일(~$...).
/// `rel`은 스캔 폴더 기준 상대 경로(파일명 포함)다. 그래서 사용자가 직접 고른 폴더는 제외되지 않는다.
pub fn is_noise(rel: &str, at_drive_root: bool) -> bool {
    let lower = rel.to_lowercase().replace('/', "\\");
    let mut parts: Vec<&str> = lower.split('\\').filter(|s| !s.is_empty()).collect();
    let Some(name) = parts.pop() else { return false; };
    if name.starts_with("~$") { return true; }
    if at_drive_root && parts.first().is_some_and(|top| SYSTEM_TOP_DIRS.contains(top)) { return true; }
    parts.iter().any(|dir| dir.starts_with('.') || *dir == "node_modules")
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ScanResult {
    pub files: BTreeMap<String, Vec<FileItem>>,
    pub skipped: Vec<String>,
    /// 스캔 폴더 자체(드라이브·최상위 폴더)를 읽지 못했는지.
    /// true면 결과가 불완전하므로 저장된 색인을 덮어쓰지 않는다.
    #[serde(default)]
    pub root_failed: bool,
    /// 스캔을 마친 시각(Unix 초).
    #[serde(default)]
    pub scanned_at: i64,
}

/// 스캔 중에 모으는 값.
#[derive(Default)]
struct Collected {
    raws: Vec<RawItem>,
    skipped: Vec<String>,
    root_failed: bool,
}

impl Collected {
    fn into_result(self) -> ScanResult {
        ScanResult {
            files: group_by_date(self.raws),
            skipped: self.skipped,
            root_failed: self.root_failed,
            scanned_at: Local::now().timestamp(),
        }
    }
}

pub fn is_drive_root(path: &str) -> bool {
    let p = path.trim_end_matches('\\');
    p.len() == 2 && p.as_bytes()[1] == b':'
}

/// 드라이브 전체는 MFT로, 일반 폴더는 폴더 순회로 읽어 하나의 결과로 합친다.
/// 드라이브 전체에 이미 포함된 폴더는 다시 읽지 않는다.
pub fn scan_all(cfg: &crate::store::Config, app: Option<&tauri::AppHandle>) -> ScanResult {
    let roots: Vec<String> = cfg.scan_paths.iter().filter(|p| is_drive_root(p)).cloned().collect();
    let folders: Vec<String> = cfg.scan_paths.iter()
        .filter(|p| !is_drive_root(p) && !path_matches(p, &roots))
        .cloned().collect();
    let mut out = Collected::default();
    read_mft(&roots, &cfg.extensions, app, &mut out);
    walk_folders(&folders, &cfg.extensions, app, &mut out);
    out.into_result()
}

/// `month`(YYYY-MM)에 속한 파일 중 `gone`이 참인 것을 결과에서 빼고, 뺀 경로를 돌려준다.
pub fn prune_month(result: &mut ScanResult, month: &str, gone: impl Fn(&str) -> bool) -> Vec<String> {
    let prefix = format!("{}-", month);
    let mut removed = Vec::new();
    for (date, files) in result.files.iter_mut() {
        if !date.starts_with(&prefix) { continue; }
        files.retain(|f| {
            let is_gone = gone(&f.path);
            if is_gone { removed.push(f.path.clone()); }
            !is_gone
        });
    }
    result.files.retain(|_, files| !files.is_empty());
    removed
}

fn emit_progress(app: Option<&tauri::AppHandle>, drive: &str, scanned: u64, total: u64, matched: u64) {
    if let Some(app) = app {
        let _ = app.emit("scan-progress", ScanProgress {
            drive: drive.to_string(), scanned, total, matched,
        });
    }
}

fn read_mft(roots: &[String], extensions: &[String], app: Option<&tauri::AppHandle>, out: &mut Collected) {
    use ntfs_reader::{file_info::FileInfo, mft::Mft, volume::Volume};

    for drive in &drives_from_paths(roots) {
        let volume_path = format!("\\\\.\\{}", drive);
        let volume = match Volume::new(&volume_path) {
            Ok(v) => v,
            Err(e) => { out.skipped.push(format!("{}: {}", drive, e)); out.root_failed = true; continue; }
        };
        let mft = match Mft::new(volume) {
            Ok(m) => m,
            Err(e) => { out.skipped.push(format!("{}: {}", drive, e)); out.root_failed = true; continue; }
        };
        let total = mft.max_record;
        let mut scanned = 0u64;
        let mut matched = 0u64;
        emit_progress(app, drive, 0, total, 0);
        let mut cache = ntfs_reader::file_info::VecCache::default();
        for file in mft.files() {
            scanned += 1;
            if scanned % 10_000 == 0 {
                emit_progress(app, drive, scanned, total, matched);
            }
            if file.is_directory() { continue; }
            let name = match file.get_best_file_name(&mft) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !ext_matches(&name, extensions) { continue; }
            let info = FileInfo::with_cache(&mft, &file, &mut cache);
            let raw_path = info.path.to_string_lossy().to_string();
            let full_path = raw_path.strip_prefix("\\\\.\\").unwrap_or(&raw_path).to_string();
            if !path_matches(&full_path, roots) { continue; }
            // "D:\폴더\파일"에서 드라이브 글자를 뗀 상대 경로로 판단한다.
            if is_noise(full_path.get(2..).unwrap_or(""), true) { continue; }
            let unix_secs = info.modified.map(|odt| odt.unix_timestamp()).unwrap_or(0);
            matched += 1;
            out.raws.push(RawItem { name, path: full_path, date_modified: unix_secs });
        }
        emit_progress(app, drive, if total > 0 { total } else { scanned }, total, matched);
    }
}

/// 폴더 순회만으로 스캔한다(테스트용).
#[cfg(test)]
fn scan_walk(cfg: &crate::store::Config) -> ScanResult {
    let mut out = Collected::default();
    walk_folders(&cfg.scan_paths, &cfg.extensions, None, &mut out);
    out.into_result()
}

fn walk_folders(folders: &[String], extensions: &[String], app: Option<&tauri::AppHandle>, out: &mut Collected) {
    let mut seen = std::collections::HashSet::new();

    for scan_path in folders {
        let mut visited = 0u64;
        let mut matched = 0u64;
        let at_drive_root = is_drive_root(scan_path);
        emit_progress(app, scan_path, 0, 0, 0);
        for entry in walkdir::WalkDir::new(scan_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // 숨김 폴더(.git, .worktrees 등) 건너뜀. 사용자가 고른 폴더 자체(depth 0)는 건너뛰지 않는다.
                let name = e.file_name().to_string_lossy();
                !(e.depth() > 0 && e.file_type().is_dir() && name.starts_with('.'))
            })
        {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    // depth 0이면 스캔 폴더 자체를 열지 못한 것이다.
                    if e.depth() == 0 { out.root_failed = true; }
                    out.skipped.push(e.to_string());
                    continue;
                }
            };
            visited += 1;
            if visited % 200 == 0 {
                emit_progress(app, scan_path, visited, 0, matched);
            }
            let path = entry.path();
            if !path.is_file() { continue; }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !ext_matches(&name, extensions) { continue; }
            let rel = path.strip_prefix(scan_path).unwrap_or(path).to_string_lossy();
            if is_noise(&rel, at_drive_root) { continue; }
            let full_path = path.to_string_lossy().to_string();
            if !seen.insert(full_path.to_lowercase().replace('/', "\\")) { continue; }
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(e) => { out.skipped.push(format!("{}: {}", full_path, e)); continue; }
            };
            let unix_secs = match metadata.modified() {
                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
                    Ok(d) => d.as_secs() as i64,
                    Err(e) => { out.skipped.push(format!("{}: {}", full_path, e)); continue; }
                },
                Err(e) => { out.skipped.push(format!("{}: {}", full_path, e)); continue; }
            };
            matched += 1;
            out.raws.push(RawItem { name, path: full_path, date_modified: unix_secs });
        }
        emit_progress(app, scan_path, visited, 0, matched);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_noise ---
    #[test]
    fn noise_recycle_bin_and_system_dirs_only_at_drive_root() {
        assert!(is_noise("$RECYCLE.BIN\\S-1-5-21\\$IABC123.pdf", true));
        assert!(is_noise("Program Files\\App\\manual.pdf", true));
        assert!(is_noise("\\Windows\\Help\\a.pdf", true));
        // 사용자가 고른 폴더 안의 같은 이름은 시스템 폴더가 아니다.
        assert!(!is_noise("Windows\\a.pdf", false));
        // 드라이브 맨 위가 아니면 제외하지 않는다.
        assert!(!is_noise("업무\\Windows\\a.pdf", true));
    }
    #[test]
    fn noise_office_temp_dot_dirs_and_node_modules() {
        assert!(is_noise("업무\\~$보고서.docx", true));
        assert!(is_noise("~$보고서.docx", false));
        assert!(is_noise("proj\\.git\\x.pdf", false));
        assert!(is_noise("proj\\node_modules\\pkg\\readme.pdf", false));
    }
    #[test]
    fn noise_keeps_ordinary_documents() {
        assert!(!is_noise("업무\\2026\\보고서.hwp", true));
        assert!(!is_noise("보고서.hwp", true));
        assert!(!is_noise("업무\\.보고서.hwp", false));
        assert!(!is_noise("", true));
    }

    // --- prune_month ---
    #[test]
    fn prune_month_removes_only_gone_files_in_that_month() {
        let item = |name: &str| FileItem { name: name.into(), path: format!("D:\\w\\{}", name), modified: 0 };
        let mut result = ScanResult::default();
        result.files.insert("2026-11-03".into(), vec![item("a.pdf"), item("gone.pdf")]);
        result.files.insert("2026-11-04".into(), vec![item("gone2.pdf")]);
        result.files.insert("2026-10-31".into(), vec![item("gone-old.pdf")]);
        let removed = prune_month(&mut result, "2026-11", |p| p.contains("gone"));
        assert_eq!(removed, vec!["D:\\w\\gone.pdf", "D:\\w\\gone2.pdf"]);
        assert_eq!(result.files["2026-11-03"].len(), 1);
        assert!(!result.files.contains_key("2026-11-04"), "빈 날짜는 지운다");
        assert_eq!(result.files["2026-10-31"].len(), 1, "다른 달은 확인하지 않는다");
    }

    // --- scan_walk: 제외 규칙과 스캔 폴더 실패 ---
    #[test]
    fn scan_walk_skips_noise_but_not_a_chosen_dot_folder() {
        let folder = std::env::temp_dir().join(format!(".everydoing-walk-noise-{}", std::process::id()));
        for sub in ["", ".git", "node_modules"] {
            std::fs::create_dir_all(folder.join(sub)).unwrap();
        }
        for file in ["work.pdf", "~$work.pdf", ".git\\hidden.pdf", "node_modules\\dep.pdf"] {
            std::fs::write(folder.join(file), "test").unwrap();
        }
        let cfg = crate::store::Config {
            scan_paths: vec![folder.to_string_lossy().to_string()],
            extensions: vec!["pdf".into()],
            ..Default::default()
        };
        let result = scan_walk(&cfg);
        let names: Vec<_> = result.files.values().flatten().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["work.pdf"]);
        assert!(!result.root_failed);
        assert!(result.scanned_at > 0);
        std::fs::remove_dir_all(folder).unwrap();
    }

    // --- scan_walk ---
    #[test]
    fn scan_walk_finds_files_in_folder() {
        use crate::store::Config;
        let folder = std::env::temp_dir().join(format!("everydoing-walk-test-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("document.pdf"), "test").unwrap();
        let cfg = Config {
            scan_paths: vec![folder.to_string_lossy().to_string()],
            extensions: vec!["pdf".to_string(), "hwp".to_string(), "docx".to_string()],
            ..Default::default()
        };
        let result = scan_walk(&cfg);
        let total: usize = result.files.values().map(|v| v.len()).sum();
        assert_eq!(total, 1);
        std::fs::remove_file(folder.join("document.pdf")).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }

    #[test]
    fn scan_walk_empty_for_wrong_ext() {
        use crate::store::Config;
        let folder = std::env::temp_dir().join(format!("everydoing-walk-ext-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("document.pdf"), "test").unwrap();
        let cfg = Config {
            scan_paths: vec![folder.to_string_lossy().to_string()],
            extensions: vec!["xyz_never_exists".to_string()],
            ..Default::default()
        };
        let result = scan_walk(&cfg);
        let total: usize = result.files.values().map(|v| v.len()).sum();
        assert_eq!(total, 0, "없는 확장자인데 파일이 나왔다");
        std::fs::remove_file(folder.join("document.pdf")).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }

    #[test]
    fn scan_walk_deduplicates_overlapping_folders() {
        let folder = std::env::temp_dir().join(format!("everydoing-walk-overlap-{}", std::process::id()));
        let child = folder.join("child");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(child.join("document.pdf"), "test").unwrap();
        let cfg = crate::store::Config {
            scan_paths: vec![folder.to_string_lossy().to_string(), child.to_string_lossy().to_string()],
            extensions: vec!["pdf".into()],
            ..Default::default()
        };
        let result = scan_walk(&cfg);
        assert_eq!(result.files.values().map(Vec::len).sum::<usize>(), 1);
        assert!(result.skipped.is_empty());
        std::fs::remove_file(child.join("document.pdf")).unwrap();
        std::fs::remove_dir(child).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }

    #[test]
    fn scan_walk_reports_missing_root() {
        let path = std::env::temp_dir().join(format!("everydoing-missing-root-{}", std::process::id()));
        let cfg = crate::store::Config {
            scan_paths: vec![path.to_string_lossy().to_string()],
            extensions: vec!["pdf".into()],
            ..Default::default()
        };
        let result = scan_walk(&cfg);
        assert!(!result.skipped.is_empty());
        assert!(result.root_failed, "스캔 폴더 자체가 없으면 불완전한 결과로 표시한다");
    }

    // --- filetime_to_date ---
    #[test]
    #[ignore]
    fn scan_bench_walkdir() {
        let cfg = crate::store::load_config().unwrap();
        println!("scan_paths: {:?}", cfg.scan_paths);
        let t = std::time::Instant::now();
        let result = super::scan_walk(&cfg);
        let elapsed = t.elapsed().as_millis();
        let total: usize = result.files.values().map(|v| v.len()).sum();
        println!("[walkdir] 완료: {}ms, 날짜 {}개, 파일 {}개",
            elapsed, result.files.len(), total);
    }

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
    #[test]
    fn group_keeps_time_and_sorts_latest_first() {
        let items = vec![
            RawItem { name: "morning.hwp".into(), path: "C:\\morning.hwp".into(), date_modified: 1781222400 },
            RawItem { name: "evening.hwp".into(), path: "C:\\evening.hwp".into(), date_modified: 1781222400 + 3600 },
        ];
        let g = group_by_date(items);
        let day = g.get("2026-06-12").unwrap();
        assert_eq!(day[0].name, "evening.hwp");
        assert_eq!(day[0].modified, 1781222400 + 3600);
        assert_eq!(day[1].name, "morning.hwp");
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
