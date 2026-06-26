# MFT 직접 인덱서 (Phase 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Everything HTTP 의존을 제거하고 ntfs-reader 크레이트로 NTFS MFT를 직접 순회해 `.exe` 단독 실행 앱을 완성한다.

**Architecture:** `everything.rs`의 순수 함수(FileItem, RawItem, filetime_to_date, group_by_date)를 새 `indexer.rs`로 이동하고, MFT 순회 로직(`drives_from_paths`, `path_matches`, `ext_matches`, `scan`)을 추가한다. `store::Config`에서 everything 필드를 제거하고, `main.rs`의 `list_files` 커맨드가 `{files, skipped}` 구조를 반환하도록 바꾼다. 프런트엔드는 새 응답 형태에 맞게 수정하고 "인덱싱 중..." 상태를 표시한다.

**Tech Stack:** Rust (Tauri 2), ntfs-reader 크레이트, Vanilla JS/HTML

## Global Constraints

- Windows 전용. `ntfs-reader`는 Windows raw volume API를 사용한다.
- `cargo test`는 순수 함수만 커버. 볼륨 순회(`scan`)는 관리자 실행으로 수동 검증.
- 기존 `store.rs` 순수 함수 테스트는 그대로 통과해야 한다.
- `reqwest` 의존은 이 단계에서 제거한다.
- 기존 `config.json`에 `everything_host`/`everything_port`가 남아있어도 `load_config()`가 패닉 없이 읽어야 한다(`#[serde(default)]`).
- 레거시 파이썬 파일은 삭제하지 않고 `legacy/` 로 이동한다(이력 보존).

---

## File Map

| 파일 | 변화 |
|------|------|
| `src-tauri/src/indexer.rs` | **신규** — 순수 함수 + MFT scan |
| `src-tauri/src/everything.rs` | **삭제** (모듈 참조 제거 후) |
| `src-tauri/src/store.rs` | Config 필드 2개 제거, `#[serde(default)]` 처리 |
| `src-tauri/src/main.rs` | mod 교체, list_files 반환 타입 변경 |
| `src-tauri/Cargo.toml` | ntfs-reader 추가, reqwest 제거 |
| `src-tauri/tauri.conf.json` | NSIS `requireAdministrator` manifest 추가 |
| `static/app.js` | list_files 응답 구조 대응, 로딩 표시, 에러 메시지 갱신 |
| `legacy/` | server.py, everything_client.py, pick_folder.py, tests/, config.json 이동 |
| `CLAUDE.md` | 인프라 설명 갱신 |

---

### Task 1: ntfs-reader 크레이트 추가 + reqwest 제거

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Interfaces:**
- Produces: `ntfs_reader` 크레이트가 `indexer.rs`에서 사용 가능한 상태

- [ ] **Step 1: ntfs-reader 버전 확인 후 Cargo.toml 수정**

`ntfs-reader`의 최신 stable 버전(0.1.x)을 추가하고, `reqwest`를 제거한다.

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
chrono = "0.4"
dirs = "5"
opener = "0.7"
rfd = "0.15"
arboard = "3"
ntfs-reader = "0.1"

[profile.release]
strip = true
```

- [ ] **Step 2: cargo fetch 로 크레이트 다운로드 확인**

```
cd src-tauri && cargo fetch
```

Expected: 에러 없이 ntfs-reader가 다운로드됨.
`ntfs-reader` 버전이 없거나 이름이 다르면 `cargo search ntfs`로 확인 후 정확한 이름/버전으로 수정.

> **주의**: `ntfs-reader`가 crates.io에 없을 경우 대안으로 `ntfs` 크레이트(Colin Finck, low-level)를 사용해야 할 수 있다. 이 경우 Task 3의 scan 구현이 달라지므로 Task 3 주석을 참고해 조정한다.

- [ ] **Step 3: cargo check 통과 확인**

```
cargo check 2>&1 | head -20
```

Expected: `everything.rs`에서 reqwest 참조 에러가 뜸 — 다음 Task에서 해결 예정.

---

### Task 2: indexer.rs 신규 생성 — 순수 함수 + 타입

**Files:**
- Create: `src-tauri/src/indexer.rs`

**Interfaces:**
- Consumes: 없음 (순수 함수)
- Produces:
  - `pub struct FileItem { pub name: String, pub path: String }` (Serialize, Clone)
  - `pub struct RawItem { pub name: String, pub path: String, pub date_modified: i64 }`
  - `pub fn filetime_to_date(ft: i64) -> Option<String>`
  - `pub fn group_by_date(results: Vec<RawItem>) -> BTreeMap<String, Vec<FileItem>>`
  - `pub fn drives_from_paths(scan_paths: &[String]) -> Vec<String>`
  - `pub fn path_matches(full_path: &str, scan_paths: &[String]) -> bool`
  - `pub fn ext_matches(name: &str, extensions: &[String]) -> bool`

- [ ] **Step 1: 실패하는 테스트 먼저 작성**

`src-tauri/src/indexer.rs` 를 만들고, 타입/함수 선언 없이 테스트 모듈만 작성한다:

```rust
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
    pub date_modified: i64,
}

// 함수 스텁 — 컴파일만 통과
pub fn filetime_to_date(_ft: i64) -> Option<String> { todo!() }
pub fn group_by_date(_: Vec<RawItem>) -> BTreeMap<String, Vec<FileItem>> { todo!() }
pub fn drives_from_paths(_: &[String]) -> Vec<String> { todo!() }
pub fn path_matches(_: &str, _: &[String]) -> bool { todo!() }
pub fn ext_matches(_: &str, _: &[String]) -> bool { todo!() }

#[cfg(test)]
mod tests {
    use super::*;

    // --- filetime_to_date ---
    #[test]
    fn filetime_known_value() {
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

    // --- group_by_date ---
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
        // "C:\Doc" 가 "C:\Document\..." 에 매칭되면 안 됨
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
```

- [ ] **Step 2: 테스트가 `todo!()` panic으로 실패하는 것 확인**

```
cd src-tauri && cargo test indexer 2>&1 | tail -20
```

Expected: 여러 테스트가 `panicked at 'not yet implemented'`로 FAIL.

- [ ] **Step 3: 순수 함수 구현**

스텁을 아래 구현으로 교체한다:

```rust
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
        let date = filetime_to_date(item.date_modified)
            .unwrap_or_else(|| "unknown".to_string());
        grouped.entry(date).or_default().push(FileItem { name: item.name, path: item.path });
    }
    grouped
}

pub fn drives_from_paths(scan_paths: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for p in scan_paths {
        // "C:\..." or "C:/..." → "C:"
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
        // prefix 매칭: full_path가 scan_path 바로 아래 또는 같은 위치여야 함
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
```

- [ ] **Step 4: 테스트 통과 확인**

```
cd src-tauri && cargo test indexer 2>&1
```

Expected: `indexer::tests` 아래 모든 테스트 PASS, 실패 0.

- [ ] **Step 5: 커밋**

```bash
git add src-tauri/src/indexer.rs src-tauri/Cargo.toml
git commit -m "feat(indexer): pure functions + unit tests (drives/path/ext/group)"
```

---

### Task 3: indexer::scan 구현 (MFT 순회)

**Files:**
- Modify: `src-tauri/src/indexer.rs`

**Interfaces:**
- Consumes: `ntfs_reader` 크레이트 API, `store::Config`
- Produces:
  - `pub struct ScanResult { pub files: BTreeMap<String, Vec<FileItem>>, pub skipped: Vec<String> }`
  - `pub fn scan(cfg: &crate::store::Config) -> ScanResult`

> **주의**: `ntfs-reader` 크레이트의 실제 API를 먼저 확인해야 한다.
> `cargo doc --open` 또는 crates.io 페이지에서 `NtfsReader`, `FileInfo` API를 확인한다.
> 아래 코드는 ntfs-reader 0.1.x 문서 기준으로 작성했으며, API가 다르면 실제 API에 맞게 조정한다.

- [ ] **Step 1: ntfs-reader API 확인**

```
cd src-tauri && cargo doc -p ntfs-reader --no-deps 2>&1 | head -5
start target/doc/ntfs_reader/index.html
```

주요 타입: `NtfsReader::new(volume_path)`, `reader.files()` 또는 유사 이터레이터, `FileInfo { name, path, modified }` 확인.

- [ ] **Step 2: ScanResult 구조체 + scan 스텁 추가**

`indexer.rs` 상단 use에 추가:
```rust
use crate::store;
```

파일 끝(테스트 모듈 앞)에 추가:
```rust
#[derive(serde::Serialize)]
pub struct ScanResult {
    pub files: BTreeMap<String, Vec<FileItem>>,
    pub skipped: Vec<String>,
}

pub fn scan(cfg: &store::Config) -> ScanResult {
    let drives = drives_from_paths(&cfg.scan_paths);
    let mut raws: Vec<RawItem> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for drive in &drives {
        // ntfs-reader: 볼륨 경로 형식 "\\.\C:" (Windows raw volume)
        let volume_path = format!("\\\\.\\{}", drive);
        match ntfs_reader::NtfsReader::new(&volume_path) {
            Ok(reader) => {
                for file_info in reader.files() {
                    let name = file_info.name.clone();
                    let path = file_info.path.clone();
                    if !ext_matches(&name, &cfg.extensions) { continue; }
                    if !path_matches(&path, &cfg.scan_paths) { continue; }
                    // ntfs-reader의 수정일은 FILETIME(i64 100ns 틱) 형태
                    let dm = file_info.modified;
                    raws.push(RawItem { name, path, date_modified: dm });
                }
            }
            Err(e) => {
                skipped.push(format!("{}: {}", drive, e));
            }
        }
    }

    ScanResult {
        files: group_by_date(raws),
        skipped,
    }
}
```

> **API 조정 가이드**: ntfs-reader의 실제 이터레이터/필드명이 다를 경우:
> - `reader.files()` → `reader.iter()` 또는 다른 이름이면 실제 메서드명으로 교체
> - `file_info.name` / `file_info.path` / `file_info.modified` → 실제 필드명으로 교체
> - `modified`가 `u64`이면 `as i64`로 캐스트

- [ ] **Step 3: cargo check 통과 확인**

```
cd src-tauri && cargo check 2>&1
```

Expected: 에러 없음 (단, `everything.rs`의 reqwest 에러는 남아있어도 됨 — 다음 Task에서 제거).

- [ ] **Step 4: 기존 순수 함수 테스트 여전히 통과 확인**

```
cd src-tauri && cargo test indexer 2>&1
```

Expected: 전부 PASS.

- [ ] **Step 5: 커밋**

```bash
git add src-tauri/src/indexer.rs
git commit -m "feat(indexer): scan() MFT traversal with ScanResult"
```

---

### Task 4: store::Config에서 Everything 필드 제거

**Files:**
- Modify: `src-tauri/src/store.rs`

**Interfaces:**
- Consumes: 없음
- Produces: `Config` 에 `everything_host`, `everything_port` 없음. 기존 config.json 호환.

- [ ] **Step 1: store.rs Config 수정**

`Config` 구조체와 `Default`에서 everything 필드를 제거한다.
기존 config.json에 두 필드가 남아있어도 `serde`가 무시하게 `#[serde(deny_unknown_fields)]`를 **붙이지 않는다** — 기본 동작이 unknown field 무시이므로 추가 속성 불필요.

```rust
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
```

- [ ] **Step 2: store 테스트 통과 확인**

```
cd src-tauri && cargo test store 2>&1
```

Expected: 기존 store 테스트 전부 PASS.

- [ ] **Step 3: 커밋**

```bash
git add src-tauri/src/store.rs
git commit -m "refactor(store): remove Everything host/port fields from Config"
```

---

### Task 5: main.rs 교체 — everything → indexer

**Files:**
- Modify: `src-tauri/src/main.rs`
- Delete reference: `everything.rs` 참조 제거

**Interfaces:**
- Consumes: `indexer::scan`, `indexer::ScanResult`, `indexer::FileItem`
- Produces: `list_files` 커맨드가 `indexer::ScanResult`(`{files, skipped}`)를 반환

- [ ] **Step 1: main.rs 수정**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;
mod indexer;

use std::collections::BTreeMap;
use indexer::{FileItem, ScanResult};

#[tauri::command]
fn list_files() -> ScanResult {
    let cfg = store::load_config();
    indexer::scan(&cfg)
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

- [ ] **Step 2: cargo build 통과 확인 (`everything.rs`는 아직 있어도 됨)**

```
cd src-tauri && cargo build 2>&1 | tail -10
```

Expected: 빌드 성공. `everything.rs`는 `mod everything;`이 없으니 컴파일 대상 아님.

- [ ] **Step 3: everything.rs 삭제**

```bash
git rm src-tauri/src/everything.rs
```

- [ ] **Step 4: cargo build 재확인**

```
cd src-tauri && cargo build 2>&1 | tail -5
```

Expected: 에러 없음.

- [ ] **Step 5: 커밋**

```bash
git add src-tauri/src/main.rs
git commit -m "feat(main): switch list_files to indexer::scan, remove everything.rs"
```

---

### Task 6: UAC 관리자 승격 설정 (tauri.conf.json)

**Files:**
- Modify: `src-tauri/tauri.conf.json`

**Interfaces:**
- Produces: NSIS 설치본 실행 시 항상 UAC 승격 요청

> Tauri 2 NSIS는 `bundle.windows.nsis.installerMode` 와 별도로 앱 실행 시 승격이 필요하다.
> Tauri 2에서 앱 manifest를 `requireAdministrator`로 설정하려면 `src-tauri/` 에 커스텀 manifest 파일을 두고 `tauri.conf.json`에서 참조한다.

- [ ] **Step 1: 커스텀 manifest 파일 생성**

`src-tauri/everydoing.exe.manifest` 를 만든다:

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
```

- [ ] **Step 2: tauri.conf.json에 manifest 참조 추가**

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
    "icon": ["icons/icon.ico"],
    "windows": {
      "nsis": {},
      "wix": {}
    }
  }
}
```

> **주의**: Tauri 2의 실제 manifest 설정 키는 버전마다 다를 수 있다.
> `cargo tauri build` 실패 시 [Tauri v2 docs — Windows manifest](https://tauri.app/v2/reference/config/) 를 확인한다.
> 개발 중(`cargo tauri dev`)에는 UAC 없이 테스트 가능하나, 볼륨 접근은 관리자로 실행해야 한다.

- [ ] **Step 3: cargo tauri build 시도 (경고 레벨 확인)**

```
cargo tauri build 2>&1 | tail -20
```

빌드 성공 여부 확인. manifest 키 오류 시 Tauri 2 문서 기준으로 수정.

- [ ] **Step 4: 커밋**

```bash
git add src-tauri/tauri.conf.json src-tauri/everydoing.exe.manifest
git commit -m "feat(bundle): requireAdministrator UAC manifest for NTFS access"
```

---

### Task 7: 프런트엔드 — 새 응답 구조 대응 + 로딩 표시

**Files:**
- Modify: `static/app.js`

**Interfaces:**
- Consumes: `list_files()` → `{ files: { [date]: FileItem[] }, skipped: string[] }`
- Produces: 달력 로딩 중 "인덱싱 중..." 표시, 건너뛴 드라이브 안내, Everything 에러 메시지 제거

현재 `app.js`의 `list_files` 호출 부분:

```js
data = await invoke("list_files");
// ...에러 시...
msg.textContent = "Everything HTTP 서버에 연결할 수 없습니다. ..."
```

이를 아래와 같이 바꾼다:

- [ ] **Step 1: app.js의 loadFiles 함수 수정**

`invoke("list_files")` 호출 전후를 찾아 수정한다.
정확한 함수/줄은 `app.js`에서 `list_files` 검색 후 확인.

변경 패턴:

```js
// 변경 전 (기존)
msg.textContent = "";
animateNext = true;
try {
  data = await invoke("list_files");
} catch (e) {
  msg.textContent = "Everything HTTP 서버에 연결할 수 없습니다. " +
    "Everything → 도구 → 옵션 → HTTP 서버를 켰는지, 포트가 설정과 맞는지 확인하세요. (" + e + ")";
}

// 변경 후
msg.textContent = "인덱싱 중...";
animateNext = true;
try {
  const result = await invoke("list_files");
  data = result.files;
  if (result.skipped && result.skipped.length > 0) {
    msg.textContent = "건너뛴 드라이브: " + result.skipped.join(", ") +
      " (NTFS가 아니거나 읽을 수 없음)";
  } else {
    msg.textContent = "";
  }
} catch (e) {
  msg.textContent = "파일 스캔 실패: " + e;
}
```

- [ ] **Step 2: `data`를 참조하는 다른 코드가 `data.files`를 직접 쓰지 않는지 확인**

```bash
grep -n "data\." static/app.js | head -30
```

`data`가 BTreeMap(Object)으로 직접 사용되는 패턴 확인. `data = result.files` 이후이므로 나머지 코드는 변경 불필요.

- [ ] **Step 3: cargo tauri dev (관리자)로 동작 확인**

관리자 터미널에서:
```
cargo tauri dev
```

- 창이 열리고 "인덱싱 중..."이 잠깐 뜬 뒤 달력에 파일이 표시되는지 확인.
- Everything을 끈 상태에서도 동일하게 동작하는지 확인.
- skipped 드라이브가 있으면 안내 문구 출력 확인.

- [ ] **Step 4: 커밋**

```bash
git add static/app.js
git commit -m "feat(frontend): handle {files, skipped} response, show indexing status"
```

---

### Task 8: 레거시 파일 정리 + CLAUDE.md 갱신

**Files:**
- Move to `legacy/`: `server.py`, `everything_client.py`, `pick_folder.py`, `tests/`, 루트 `config.json`
- Modify: `CLAUDE.md`

- [ ] **Step 1: legacy/ 폴더로 이동**

```bash
mkdir legacy
git mv server.py legacy/
git mv everything_client.py legacy/
git mv pick_folder.py legacy/
git mv tests legacy/
git mv config.json legacy/
```

> 루트 `config.json`은 Python 서버용. Tauri 앱의 설정은 `%APPDATA%\EveryDoing\config.json`에 저장되므로 루트 파일은 레거시.

- [ ] **Step 2: CLAUDE.md 인프라 표 갱신**

`CLAUDE.md`의 인프라/배포 구조 표와 결정사항을 아래로 교체:

```markdown
## 인프라 / 배포 구조
| 구분 | 내용 |
|------|------|
| 인덱스 엔진 | ntfs-reader 크레이트로 NTFS MFT 직접 순회 (Everything 불필요) |
| 백엔드 | Tauri 2 Rust 백엔드 (`src-tauri/src/`) |
| 프런트 | 바닐라 HTML/CSS/JS (`static/`) |
| 실행 | `cargo tauri dev` (관리자) → 데스크톱 창 |
| 배포 | `cargo tauri build` → NSIS 설치본 (UAC 승격 포함) |
| 플랫폼 | Windows 전용 (NTFS MFT 원시 접근) |
| 설정 파일 | `%APPDATA%\EveryDoing\config.json` (앱이 관리) |

## 프로젝트 결정사항 (왜 이렇게 되어 있는지)
- MFT 직접 인덱싱 (ntfs-reader) — Everything 의존 제거, .exe 단독 실행.
- 시작 시 전체 한 번 스캔 + "새로고침" 재스캔. USN journal 실시간 감시 없음.
- 기준 날짜는 파일 수정일. FILETIME(1601 기준 100ns 틱)을 변환해 사용.
- scan_paths에서 드라이브 문자를 자동 추출해 해당 볼륨만 인덱싱.
- 비NTFS / 권한 실패 드라이브는 skipped 목록에 기록하고 계속 진행.
- 설정은 `%APPDATA%\EveryDoing\config.json` (scan_paths·extensions).
- 달력 칸 = "다이어리 칸": 날짜 + 우상단 파일 수 + 확장자 종류칩 + 본문 한 줄 메모.
- 메모는 `%APPDATA%\EveryDoing\memos.json`에 날짜별 로컬 저장.
- 레거시 Python 코드는 `legacy/`에 보관.

## 자주 막히는 것들 (시행착오 메모)
- MFT 접근에는 관리자 권한 필요. `cargo tauri dev`도 관리자 터미널에서 실행.
- 비NTFS 드라이브(USB FAT32 등)는 자동으로 skipped 처리됨.

## 코드 수정 후 체크리스트
- `cargo test` (src-tauri/) 통과 확인
- 관리자 터미널에서 `cargo tauri dev` 후 달력에 파일 뜨는지 확인
```

- [ ] **Step 3: cargo test 전체 통과 최종 확인**

```
cd src-tauri && cargo test 2>&1
```

Expected: 모든 테스트 PASS.

- [ ] **Step 4: 커밋**

```bash
git add legacy/ CLAUDE.md
git commit -m "chore: move legacy Python files to legacy/, update CLAUDE.md for MFT indexer"
```

---

## 수동 검증 시나리오 (관리자 실행 필요)

Task 7 Step 3에서 수행하며, 아래를 순서대로 확인:

1. 관리자 터미널 → `cargo tauri dev` 실행 → UAC 창 또는 관리자 권한 확인
2. scan_paths의 드라이브 파일들이 달력에 표시됨 (Everything 없이)
3. 새 파일 생성 후 "새로고침" 버튼 → 달력에 반영
4. 비NTFS 드라이브를 scan_path에 추가 → "건너뛴 드라이브" 안내 표시
5. Everything 완전히 종료 상태에서도 동일 동작

---

## Self-Review

**스펙 커버리지 체크:**
- [x] ntfs-reader 볼륨 순회 → Task 3
- [x] drives_from_paths / path_matches / ext_matches 순수 함수 + TDD → Task 2
- [x] ScanResult {files, skipped} 반환 → Task 3, 5
- [x] Config에서 everything 필드 제거 → Task 4
- [x] requireAdministrator UAC → Task 6
- [x] 프런트 "인덱싱 중..." + skipped 안내 → Task 7
- [x] Everything 레거시 정리 → Task 5(everything.rs 삭제), Task 8
- [x] CLAUDE.md 갱신 → Task 8
- [x] scan_paths 비어있음 → `drives_from_paths` 빈 배열 반환 → scan이 빈 ScanResult 반환 → 기존 빈 상태 UI 재사용 (별도 처리 불필요)
- [x] 모든 볼륨 실패 시 → skipped에 전부 기록, files 비어있음 → 프런트가 "건너뛴 드라이브" 표시

**플레이스홀더 스캔:** 없음. 모든 코드 블록이 실제 구현을 포함.

**타입 일관성:**
- `FileItem { name: String, path: String }` — Task 2에서 정의, Task 5 main.rs에서 import
- `ScanResult { files: BTreeMap<String, Vec<FileItem>>, skipped: Vec<String> }` — Task 3에서 정의, Task 5 main.rs에서 반환, Task 7 JS에서 `result.files` / `result.skipped` 로 소비
- `RawItem { name, path, date_modified: i64 }` — Task 2/3 내부 전용
