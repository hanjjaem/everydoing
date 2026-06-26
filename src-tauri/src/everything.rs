use std::collections::BTreeMap;
use serde::Serialize;
use chrono::{TimeZone, Local};
use crate::store;

const FILETIME_POSIX_OFFSET: i64 = 11_644_473_600; // 1601→1970 초

#[derive(Serialize, Clone)]
pub struct FileItem {
    pub name: String,
    pub path: String,
}

pub struct RawItem {
    pub name: String,
    pub path: String,
    pub date_modified: i64,
}

pub fn build_search_query(scan_paths: &[String], extensions: &[String]) -> String {
    let ext_part = format!("ext:{}", extensions.join(";"));
    let quoted: Vec<String> = scan_paths
        .iter()
        .map(|p| format!("\"{}\\\"", p.trim_end_matches('\\')))
        .collect();
    format!("{} ({})", ext_part, quoted.join(" | "))
}

pub fn filetime_to_date(ft: i64) -> Option<String> {
    if ft <= 0 { return None; }
    let secs = ft / 10_000_000 - FILETIME_POSIX_OFFSET;
    match Local.timestamp_opt(secs, 0).single() {
        Some(dt) => Some(dt.format("%Y-%m-%d").to_string()),
        None => None,
    }
}

pub fn group_by_date(results: Vec<RawItem>) -> BTreeMap<String, Vec<FileItem>> {
    let mut grouped: BTreeMap<String, Vec<FileItem>> = BTreeMap::new();
    for item in results {
        let date = filetime_to_date(item.date_modified).unwrap_or_else(|| "unknown".to_string());
        grouped.entry(date).or_default().push(FileItem { name: item.name, path: item.path });
    }
    grouped
}

pub fn fetch_files(cfg: &store::Config) -> Result<BTreeMap<String, Vec<FileItem>>, String> {
    let query = build_search_query(&cfg.scan_paths, &cfg.extensions);
    let url = format!("http://{}:{}/", cfg.everything_host, cfg.everything_port);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .query(&[
            ("search", query.as_str()),
            ("json", "1"),
            ("path_column", "1"),
            ("date_modified_column", "1"),
        ])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .map_err(|e| e.to_string())?;
    let body: serde_json::Value = resp.json().map_err(|e| e.to_string())?;
    let mut raws = Vec::new();
    if let Some(arr) = body.get("results").and_then(|v| v.as_array()) {
        for r in arr {
            let name = r.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let path = r.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let dm = r.get("date_modified")
                .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_i64().map(|n| n.to_string())))
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            let full = if path.is_empty() { name.clone() } else { format!("{}\\{}", path, name) };
            raws.push(RawItem { name, path: full, date_modified: dm });
        }
    }
    Ok(group_by_date(raws))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_combines_ext_and_paths() {
        let q = build_search_query(
            &["C:\\Users\\u\\Desktop".into(), "C:\\Users\\u\\Docs".into()],
            &["hwp".into(), "docx".into()],
        );
        assert_eq!(q, "ext:hwp;docx (\"C:\\Users\\u\\Desktop\\\" | \"C:\\Users\\u\\Docs\\\")");
    }

    #[test]
    fn query_single_path() {
        let q = build_search_query(&["C:\\A".into()], &["pdf".into()]);
        assert_eq!(q, "ext:pdf (\"C:\\A\\\")");
    }

    #[test]
    fn filetime_known_value() {
        // 2026-06-12 00:00:00 UTC == FILETIME 134256960000000000 → local KST date 2026-06-12
        assert_eq!(filetime_to_date(134256960000000000), Some("2026-06-12".to_string()));
    }

    #[test]
    fn filetime_zero_is_none() {
        assert_eq!(filetime_to_date(0), None);
    }

    #[test]
    fn group_buckets_and_unknown() {
        let items = vec![
            RawItem { name: "a.hwp".into(), path: "C:\\x\\a.hwp".into(), date_modified: 134256960000000000 },
            RawItem { name: "c.pdf".into(), path: "C:\\c.pdf".into(), date_modified: 0 },
        ];
        let g = group_by_date(items);
        assert_eq!(g.get("2026-06-12").unwrap().len(), 1);
        assert_eq!(g.get("unknown").unwrap()[0].name, "c.pdf");
    }
}
