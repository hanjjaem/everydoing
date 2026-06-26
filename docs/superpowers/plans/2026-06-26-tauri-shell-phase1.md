# Tauri Desktop Shell (Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 현재 웹앱(`server.py` + `static/`)을 Tauri 데스크톱 앱으로 전환하되, 파일 목록은 1단계에선 Everything HTTP를 그대로 호출한다(기능 동등).

**Architecture:** Rust 백엔드(`src-tauri/`)가 기존 Python API 7개를 Tauri 커맨드로 제공한다. 순수 로직(검색쿼리·FILETIME변환·날짜그룹화·확장자정규화·메모셋·config검증)은 `cargo test`로 TDD하고, 커맨드 와이어링과 프런트 `invoke` 연동은 `cargo tauri dev`로 수동 검증한다. 데이터 파일은 `%APPDATA%\EveryDoing\`에 저장.

**Tech Stack:** Tauri v2, Rust 1.95, reqwest(blocking), serde/serde_json, chrono, dirs, opener, rfd, arboard. 프런트는 기존 바닐라 JS(빌드 단계 없음, `withGlobalTauri`).

## Global Constraints

- 플랫폼: Windows 전용. 기존 기능 동등(feature parity), 새 기능 없음.
- 데이터 파일은 `%APPDATA%\EveryDoing\` (`config.json`, `memos.json`). repo 루트의 동명 파일은 런타임에서 미사용.
- 기본 config: `everything_host="localhost"`, `everything_port=8080`, `scan_paths=[]`, `extensions=["hwp","hwpx","doc","docx","xls","xlsx","ppt","pptx","pdf"]`.
- 검색 쿼리 형식 보존: `ext:hwp;docx ("C:\\path\\" | "D:\\path\\")` (경로 끝 백슬래시 정규화).
- FILETIME→로컬 `YYYY-MM-DD`, 0/파싱불가는 제외, 변환실패분은 `"unknown"` 버킷.
- 확장자 정규화: 앞 `.` 1개 제거, 소문자, 중복제거·순서보존, 빈값 드롭.
- config 검증: `scan_paths` 비면 거부, `extensions` 정규화 후 비면 거부.
- Tauri 커맨드 인자: JS는 camelCase로 전달(`scanPaths`) → Rust snake_case(`scan_paths`) 자동 매핑.
- 프런트는 빌드 단계 없음: `tauri.conf.json`의 `app.withGlobalTauri=true`, `build.frontendDist="../static"`.
- 커밋 메시지 끝에: `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`

---

### Task 0 (사용자 선행): Tauri CLI 설치

환경 설치는 사용자가 직접 실행한다(전역 규칙). 구현 시작 전 한 번:

```
npm install -g @tauri-apps/cli
cargo tauri --version   # 동작 확인
```

(또는 `cargo install tauri-cli --version "^2"` 후 `cargo tauri --version`.)

---

### Task 1: Tauri 셸 스캐폴딩 (창이 뜨고 기존 UI 렌더)

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/icons/icon.ico` (Tauri 기본 아이콘 사용)

**Interfaces:**
- Produces: 빌드 가능한 Tauri 앱. `cargo tauri dev`가 `static/index.html`을 로드하는 창을 띄움.

- [ ] **Step 1: Cargo.toml 작성**

`src-tauri/Cargo.toml`:
```toml
[package]
name = "everydoing"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1", features = ["preserve_order"] }
reqwest = { version = "0.12", features = ["blocking", "json"] }
chrono = "0.4"
dirs = "5"
opener = "0.7"
rfd = "0.15"
arboard = "3"

[profile.release]
strip = true
```

- [ ] **Step 2: tauri.conf.json 작성**

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "EveryDoing",
  "version": "0.1.0",
  "identifier": "kr.everydoing.app",
  "build": { "frontendDist": "../static" },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      { "title": "EveryDoing", "width": 1280, "height": 840, "minWidth": 900, "minHeight": 600 }
    ],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis"],
    "icon": ["icons/icon.ico"]
  }
}
```

- [ ] **Step 3: build.rs 작성**

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 4: 최소 main.rs 작성 (커맨드는 다음 Task에서 추가)**

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 기본 아이콘 생성**

Tauri CLI로 기본 아이콘 세트 생성(임시):
```
cd src-tauri
cargo tauri icon
```
(브랜드 아이콘은 범위 밖. 기본 생성물 사용. `icons/icon.ico` 존재 확인.)

- [ ] **Step 6: 창 실행 확인**

Run: `cd src-tauri && cargo tauri dev`
Expected: 창이 뜨고 달력 UI가 렌더됨(파일 데이터는 아직 없어 빈 달력 + `#msg`에 연결 오류 문구). 콘솔에 치명적 패닉 없음.

- [ ] **Step 7: 커밋**

```bash
git add src-tauri/
git commit -m "feat(tauri): scaffold desktop shell loading static frontend"
```

---

### Task 2: store.rs — config/memos 저장소 + 순수 로직 (TDD)

**Files:**
- Create: `src-tauri/src/store.rs`
- Modify: `src-tauri/src/main.rs` (모듈 선언 `mod store;`)

**Interfaces:**
- Produces:
  - `fn normalize_extension(raw: &str) -> Option<String>`
  - `fn normalize_extensions(raws: &[String]) -> Vec<String>`
  - `fn set_memo(memos: &BTreeMap<String,String>, date: &str, text: &str) -> BTreeMap<String,String>`
  - `fn validate_config(scan_paths: Vec<String>, extensions: Vec<String>) -> Result<(Vec<String>, Vec<String>), String>`
  - `struct Config { everything_host, everything_port, scan_paths, extensions }` (Serialize/Deserialize)
  - `fn data_dir() -> PathBuf`, `fn load_config() -> Config`, `fn save_config(Config) -> Result<(),String>`,
    `fn load_memos() -> BTreeMap<String,String>`, `fn save_memos(&BTreeMap<String,String>) -> Result<(),String>`

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/store.rs` (파일 생성, 먼저 테스트만):
```rust
use std::collections::BTreeMap;

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
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `cd src-tauri && cargo test store::`
Expected: 컴파일 에러(함수 미정의) — FAIL.

- [ ] **Step 3: 최소 구현 작성**

`src-tauri/src/store.rs` 상단(테스트 모듈 위)에:
```rust
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
    pub everything_host: String,
    pub everything_port: u16,
    pub scan_paths: Vec<String>,
    pub extensions: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            everything_host: "localhost".into(),
            everything_port: 8080,
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
```

`src-tauri/src/main.rs` 맨 위에 `mod store;` 추가.

- [ ] **Step 4: 테스트 통과 확인**

Run: `cd src-tauri && cargo test store::`
Expected: 8 passed.

- [ ] **Step 5: 커밋**

```bash
git add src-tauri/src/store.rs src-tauri/src/main.rs
git commit -m "feat(tauri): store module - config/memos IO + pure helpers (TDD)"
```

---

### Task 3: everything.rs — Everything 쿼리 + 순수 변환 (TDD)

**Files:**
- Create: `src-tauri/src/everything.rs`
- Modify: `src-tauri/src/main.rs` (`mod everything;`)

**Interfaces:**
- Consumes: `store::Config` (Task 2).
- Produces:
  - `struct FileItem { name: String, path: String }` (Serialize)
  - `fn build_search_query(scan_paths: &[String], extensions: &[String]) -> String`
  - `fn filetime_to_date(ft: i64) -> Option<String>`
  - `fn group_by_date(results: Vec<RawItem>) -> BTreeMap<String, Vec<FileItem>>` where `RawItem { name, path, date_modified: i64 }`
  - `fn fetch_files(cfg: &store::Config) -> Result<BTreeMap<String, Vec<FileItem>>, String>`

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/everything.rs` (테스트 먼저):
```rust
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
```

(주의: `filetime_known_value`는 로컬 타임존 의존. 개발/CI가 KST가 아니면 기대 날짜가 달라질 수 있어, 이 테스트는 KST 환경 기준이다. 타임존 비의존 검증이 필요하면 UTC 고정 변환으로 바꾼다 — 아래 구현은 로컬 타임존 사용으로 기존 Python 동작과 동일하게 맞춘다.)

- [ ] **Step 2: 테스트 실패 확인**

Run: `cd src-tauri && cargo test everything::`
Expected: 컴파일 에러 — FAIL.

- [ ] **Step 3: 최소 구현 작성**

`src-tauri/src/everything.rs` 상단:
```rust
use std::collections::BTreeMap;
use serde::Serialize;
use chrono::{TimeZone, Local};

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
    use crate::store;
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
```

`crate::store`를 참조하므로 `main.rs`에 `mod everything;` 추가(`mod store;` 다음).
(주의: Everything의 `path` 컬럼은 디렉터리 경로다. 전체 파일 경로 = `path\name`로 합친다 — 기존 Python은 Everything이 path에 풀패스를 넣는 설정에 따라 달랐을 수 있으니, Task 5 수동검증에서 "폴더 열기/경로 복사"가 올바른 풀패스인지 확인하고 필요시 합성 방식만 조정한다.)

- [ ] **Step 4: 테스트 통과 확인**

Run: `cd src-tauri && cargo test everything::`
Expected: 5 passed. (KST 환경 기준; 다른 타임존이면 `filetime_known_value` 기대값 조정.)

- [ ] **Step 5: 커밋**

```bash
git add src-tauri/src/everything.rs src-tauri/src/main.rs
git commit -m "feat(tauri): everything module - query/filetime/grouping + fetch (TDD)"
```

---

### Task 4: 커맨드 와이어링 (main.rs)

**Files:**
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `store::*` (Task 2), `everything::*` (Task 3).
- Produces (Tauri 커맨드, 프런트에서 `invoke`):
  - `list_files() -> Result<BTreeMap<String, Vec<FileItem>>, String>`
  - `open_path(path: String, mode: String) -> Result<(), String>`
  - `pick_folder() -> Result<Option<String>, String>`
  - `get_config() -> Result<store::Config, String>`
  - `save_config(scan_paths: Vec<String>, extensions: Vec<String>) -> Result<(), String>`
  - `get_memos() -> Result<BTreeMap<String,String>, String>`
  - `save_memo(date: String, text: String) -> Result<(), String>`
  - `copy_text(text: String) -> Result<(), String>`

- [ ] **Step 1: main.rs에 커맨드 구현 + 등록**

`src-tauri/src/main.rs` 전체:
```rust
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
```

- [ ] **Step 2: 컴파일 + 기존 테스트 유지 확인**

Run: `cd src-tauri && cargo test`
Expected: store(8) + everything(5) 테스트 통과, 컴파일 성공.

- [ ] **Step 3: 커밋**

```bash
git add src-tauri/src/main.rs
git commit -m "feat(tauri): register commands (files, open, pick, config, memos, clipboard)"
```

---

### Task 5: 프런트 invoke 전환 + 창 채움 CSS

**Files:**
- Modify: `static/app.js` (fetch→invoke, 7 군데 + clipboard)
- Modify: `static/index.html` (창 채움 CSS)

**Interfaces:**
- Consumes: Task 4 커맨드들 via `window.__TAURI__.core.invoke`.

- [ ] **Step 1: app.js 상단에 invoke 헬퍼 추가**

`static/app.js` 맨 위(`let data = {};` 앞)에:
```javascript
const invoke = window.__TAURI__.core.invoke;
```

- [ ] **Step 2: list_files 전환 (`app.js` load 내 `fetch("/api/files")`)**

`load()`의 try 블록을 교체:
```javascript
  try {
    data = await invoke("list_files");
  } catch (e) {
    msg.textContent = "Everything HTTP 서버에 연결할 수 없습니다. " +
      "Everything → 도구 → 옵션 → HTTP 서버를 켰는지, 포트가 설정과 맞는지 확인하세요. (" + e + ")";
    data = {};
  }
```

- [ ] **Step 3: memos/config 로드 전환 (`load()` 후반)**

기존 두 블록을 교체:
```javascript
  try { memos = await invoke("get_memos"); } catch (_) { memos = {}; }
  try { scanPaths = (await invoke("get_config")).scan_paths || []; } catch (_) { scanPaths = []; }
  renderAll();
```

- [ ] **Step 4: save_memo 전환 (`saveMemo` 내 `fetch("/api/memos", ...)`)**

```javascript
  try { await invoke("save_memo", { date: cellKey, text: trimmed }); } catch (_) {}
```

- [ ] **Step 5: 경로 복사 전환 (`navigator.clipboard.writeText`)**

타임라인 "경로" 버튼 핸들러를 교체:
```javascript
      acts.appendChild(actionBtn("경로", e => { e.stopPropagation(); invoke("copy_text", { text: f.path }); }));
```

- [ ] **Step 6: open_path 전환 (`openFile` 내 `fetch("/api/open", ...)`)**

`openFile` 함수 본문을 교체:
```javascript
async function openFile(path, mode) {
  try {
    await invoke("open_path", { path, mode });
  } catch (e) {
    alert(e);
  }
}
```

- [ ] **Step 7: 설정 패널 config 로드/저장 + 폴더 선택 전환**

`openPanel` 내 `fetch("/api/config")`:
```javascript
  const cfg = await invoke("get_config");
```
`addPath` 내 `fetch("/api/pick-folder", ...)`:
```javascript
  let path = null;
  try { path = await invoke("pick_folder"); } catch (_) {}
  if (path && !editPaths.includes(path)) { editPaths.push(path); renderPaths(); }
```
`savePaths` 내 `fetch("/api/config", {POST...})`:
```javascript
  try {
    await invoke("save_config", { scanPaths: editPaths, extensions: editExts });
  } catch (e) {
    document.getElementById("panelMsg").textContent = e;
    return;
  }
  closePanel();
  load();
```

- [ ] **Step 8: index.html — 창 채움 CSS**

`static/index.html`의 `body` 규칙에서 가짜 여백/그라데이션 제거하고, `.app`이 창을 꽉 채우게 교체:
```css
  body {
    font-family: var(--sans);
    margin: 0; padding: 0;
    color: var(--ink); background: var(--bg);
    font-size: 15px; line-height: 1.45;
    -webkit-font-smoothing: antialiased; text-rendering: optimizeLegibility;
  }
  .app {
    height: 100vh; max-width: none; margin: 0;
    display: flex; flex-direction: column;
    background: var(--card); border: none; border-radius: 0;
    box-shadow: none; overflow: hidden;
  }
```

- [ ] **Step 9: 전체 동작 수동 검증 (`cargo tauri dev`)**

Run: `cd src-tauri && cargo tauri dev`
Everything HTTP 서버 켠 상태에서 확인:
1. 창이 화면을 꽉 채우고 달력이 렌더됨.
2. 파일 있는 날에 칩/개수 표시(`list_files` 동작), 기존 웹앱과 동일 결과.
3. 메모 입력→저장 후 `%APPDATA%\EveryDoing\memos.json` 생성, 재시작 후 유지.
4. 설정에서 "폴더 추가"→네이티브 선택창, 확장자 저장→`config.json` 반영.
5. 타임라인에서 파일 열기/폴더 열기/경로 복사 동작(풀패스 정확).
6. Everything 끈 상태에서 `#msg` 안내 문구 표시.

(검증 중 "폴더 열기/경로"가 잘못된 경로면 `everything.rs`의 경로 합성(`path\name`)만 조정 후 재확인.)

- [ ] **Step 10: 커밋**

```bash
git add static/app.js static/index.html
git commit -m "feat(tauri): frontend invoke migration + window-fill layout"
```

---

### Task 6: 설치 파일 빌드 검증

**Files:** 없음(빌드 산출물).

- [ ] **Step 1: 릴리스 빌드**

Run: `cd src-tauri && cargo tauri build`
Expected: 빌드 성공, `src-tauri/target/release/bundle/nsis/`에 `EveryDoing_0.1.0_x64-setup.exe` 생성.

- [ ] **Step 2: 설치 후 실행 확인**

생성된 설치파일로 설치 → 시작메뉴에서 EveryDoing 실행 → 창이 뜨고(Everything 켠 상태) 달력·메모·설정 동작 확인.

- [ ] **Step 3: gitignore + 커밋**

`.gitignore`에 Rust/Tauri 산출물 추가:
```
# Rust / Tauri 빌드 산출물
src-tauri/target/
```
```bash
git add .gitignore
git commit -m "chore: gitignore tauri build artifacts"
```

---

## Self-Review 결과

- **Spec 커버리지:** 구조=Task1, store(config/memos/검증/정규화/메모)=Task2, everything(쿼리/FILETIME/그룹/fetch)=Task3, 커맨드 7개+clipboard=Task4, 프런트 invoke 전환 7군데+창채움=Task5, 빌드/배포=Task6, APPDATA 데이터위치=Task2(data_dir). 기본 config 값·검증 규칙·쿼리 형식·FILETIME 규칙 모두 Global Constraints + Task2/3에 반영.
- **플레이스홀더:** 없음(모든 코드 블록 실제 내용). "필요시 조정"은 검증 단계의 조건부 확인이며 구현 누락이 아님.
- **타입 일관성:** `FileItem{name,path}`·`Config`·`set_memo`/`validate_config`/`normalize_extensions` 시그니처가 Task2/3 정의와 Task4 사용처 일치. `list_files`/`save_config`/`save_memo` 반환·인자가 프런트 invoke(Task5)와 일치(camelCase `scanPaths`↔`scan_paths`).
- **주의(검증 단계 처리):** ① `filetime_known_value` 테스트는 로컬 타임존(KST) 의존 — 다른 타임존이면 기대값 조정. ② Everything `path` 컬럼의 풀패스 합성(`path\name`)은 Task5 수동검증에서 확인 후 필요시 조정.
