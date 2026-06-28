# 캐시 + 프로그레스바 구현 플랜

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 스캔 결과를 index.json에 캐시해 재시작 시 즉시 로드하고, 스캔 중 Tauri 이벤트로 프로그레스바를 표시한다.

**Architecture:** Rust에서 스캔 진행률을 `scan-progress` 이벤트로 emit하고 완료 시 `scan-done`으로 결과를 전달한다. 결과는 `index.json`에 저장되며 다음 실행 시 즉시 로드된다. 프론트는 캐시 로드 → (없으면) 스캔 시작 → 프로그레스바 → 달력 표시 순으로 동작한다.

**Tech Stack:** Tauri 2, Rust, Vanilla JS. `tauri::AppHandle`, `app.emit()`.

## Global Constraints

- Tauri 2 사용 (`tauri = { version = "2" }`)
- Windows 전용, NTFS MFT 직접 접근
- 캐시 경로: `%APPDATA%\EveryDoing\index.json`
- 관리자 터미널에서 `cargo tauri dev` 실행 필요
- `ntfs_reader`의 `info.path`는 `\\.\D:\...` 형태 → `strip_prefix("\\\\.\\")` 정규화 필수

---

### Task 1: store.rs — 캐시 저장/로드 함수 추가

**Files:**
- Modify: `src-tauri/src/store.rs`

**Interfaces:**
- Produces:
  - `save_index(cfg: &Config, result: &ScanResult) -> Result<(), String>`
  - `load_index(cfg: &Config) -> Option<ScanResult>`

- [ ] **Step 1: 테스트 작성**

`src-tauri/src/store.rs` 하단 `#[cfg(test)]` 블록에 추가:

```rust
#[test]
fn index_roundtrip() {
    use std::collections::BTreeMap;
    use crate::indexer::{ScanResult, FileItem};
    let cfg = Config {
        scan_paths: vec!["D:\\test".to_string()],
        extensions: vec!["pdf".to_string()],
    };
    let mut files = BTreeMap::new();
    files.insert("2026-06-29".to_string(), vec![FileItem {
        name: "a.pdf".to_string(),
        path: "D:\\test\\a.pdf".to_string(),
    }]);
    let result = ScanResult { files, skipped: vec![] };
    save_index(&cfg, &result).unwrap();
    let loaded = load_index(&cfg).unwrap();
    assert_eq!(loaded.files["2026-06-29"][0].name, "a.pdf");
}

#[test]
fn index_invalidated_when_paths_differ() {
    use crate::indexer::ScanResult;
    use std::collections::BTreeMap;
    let cfg_a = Config {
        scan_paths: vec!["D:\\a".to_string()],
        extensions: vec!["pdf".to_string()],
    };
    let cfg_b = Config {
        scan_paths: vec!["D:\\b".to_string()],
        extensions: vec!["pdf".to_string()],
    };
    let result = ScanResult { files: BTreeMap::new(), skipped: vec![] };
    save_index(&cfg_a, &result).unwrap();
    assert!(load_index(&cfg_b).is_none());
}
```

- [ ] **Step 2: 테스트 실패 확인**

```
cd src-tauri
cargo test index_roundtrip index_invalidated -- --test-threads=1
```
Expected: `error[E0425]: cannot find function \`save_index\``

- [ ] **Step 3: IndexCache 구조체 및 함수 구현**

`src-tauri/src/store.rs`에서 `use` 블록 아래에 추가:

```rust
use crate::indexer::ScanResult;

#[derive(Serialize, Deserialize)]
struct IndexCache {
    scan_paths: Vec<String>,
    extensions: Vec<String>,
    #[serde(flatten)]
    result: ScanResult,
}

fn index_path() -> PathBuf { let mut p = data_dir(); p.push("index.json"); p }

pub fn save_index(cfg: &Config, result: &ScanResult) -> Result<(), String> {
    let cache = IndexCache {
        scan_paths: cfg.scan_paths.clone(),
        extensions: cfg.extensions.clone(),
        result: result.clone(),
    };
    let s = serde_json::to_string_pretty(&cache).map_err(|e| e.to_string())?;
    std::fs::write(index_path(), s).map_err(|e| e.to_string())
}

pub fn load_index(cfg: &Config) -> Option<ScanResult> {
    let s = std::fs::read_to_string(index_path()).ok()?;
    let cache: IndexCache = serde_json::from_str(&s).ok()?;
    if cache.scan_paths != cfg.scan_paths || cache.extensions != cfg.extensions {
        return None;
    }
    Some(cache.result)
}
```

`ScanResult`와 `FileItem`에 `Clone`, `Deserialize` derive 추가 (`src-tauri/src/indexer.rs`):

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct FileItem { ... }

#[derive(Serialize, Deserialize, Clone)]
pub struct ScanResult { ... }
```

- [ ] **Step 4: 테스트 통과 확인**

```
cargo test index_roundtrip index_invalidated -- --test-threads=1
```
Expected: `test result: ok. 2 passed`

- [ ] **Step 5: 커밋**

```
git add src-tauri/src/store.rs src-tauri/src/indexer.rs
git commit -m "feat: index.json 캐시 저장/로드 (store.rs)"
```

---

### Task 2: indexer.rs — AppHandle 받아 진행률 emit

**Files:**
- Modify: `src-tauri/src/indexer.rs`

**Interfaces:**
- Consumes: `tauri::AppHandle`
- Produces:
  - `scan(cfg: &Config, app: &tauri::AppHandle) -> ScanResult`
  - emit 이벤트 `"scan-progress"`: `{ drive: String, scanned: u64, total: u64, matched: u64 }`
  - emit 이벤트 `"scan-done"`: `ScanResult`

- [ ] **Step 1: ScanProgress 구조체 추가**

`src-tauri/src/indexer.rs` 상단 구조체 영역에 추가:

```rust
#[derive(Serialize, Clone)]
pub struct ScanProgress {
    pub drive: String,
    pub scanned: u64,
    pub total: u64,
    pub matched: u64,
}
```

- [ ] **Step 2: scan() 시그니처 변경 및 emit 추가**

기존 `pub fn scan(cfg: &crate::store::Config) -> ScanResult` 를 아래로 교체:

```rust
pub fn scan(cfg: &crate::store::Config, app: &tauri::AppHandle) -> ScanResult {
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
        let total = mft.file_count() as u64;
        let mut scanned = 0u64;
        let mut matched = 0u64;
        let mut cache = ntfs_reader::file_info::VecCache::default();
        for file in mft.files() {
            scanned += 1;
            if file.is_directory() { continue; }
            let name = match file.get_best_file_name(&mft) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if !ext_matches(&name, &cfg.extensions) { continue; }
            let info = FileInfo::with_cache(&mft, &file, &mut cache);
            let raw_path = info.path.to_string_lossy().to_string();
            let full_path = raw_path.strip_prefix("\\\\.\\").unwrap_or(&raw_path).to_string();
            if !path_matches(&full_path, &cfg.scan_paths) { continue; }
            let unix_secs = info.modified.map(|odt| odt.unix_timestamp()).unwrap_or(0);
            matched += 1;
            raws.push(RawItem { name, path: full_path, date_modified: unix_secs });

            if scanned % 10_000 == 0 {
                let _ = app.emit("scan-progress", ScanProgress {
                    drive: drive.clone(),
                    scanned,
                    total,
                    matched,
                });
            }
        }
        let _ = app.emit("scan-progress", ScanProgress {
            drive: drive.clone(),
            scanned,
            total,
            matched,
        });
    }

    ScanResult { files: group_by_date(raws), skipped }
}
```

- [ ] **Step 3: 컴파일 확인**

```
cargo build 2>&1 | head -30
```
Expected: warning 있어도 error 없음

- [ ] **Step 4: 커밋**

```
git add src-tauri/src/indexer.rs
git commit -m "feat: scan()에 AppHandle 추가, 진행률 emit"
```

---

### Task 3: main.rs — load_cache / start_scan 커맨드로 분리

**Files:**
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: `store::load_index`, `store::save_index`, `indexer::scan`
- Produces:
  - `load_cache() -> Option<ScanResult>` — Tauri command
  - `start_scan(app: AppHandle)` — Tauri command, 백그라운드 실행 후 `scan-done` emit

- [ ] **Step 1: list_files 제거, load_cache / start_scan 추가**

`src-tauri/src/main.rs` 전체를 아래로 교체:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod store;
mod indexer;

use std::collections::BTreeMap;
use indexer::ScanResult;
use tauri::AppHandle;

#[tauri::command]
fn load_cache() -> Option<ScanResult> {
    let cfg = store::load_config();
    store::load_index(&cfg)
}

#[tauri::command]
async fn start_scan(app: AppHandle) {
    let cfg = store::load_config();
    let result = tauri::async_runtime::spawn_blocking({
        let app2 = app.clone();
        move || indexer::scan(&cfg, &app2)
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
            load_cache, start_scan,
            open_path, pick_folder,
            get_config, save_config,
            get_memos, save_memo, copy_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: 컴파일 확인**

```
cargo build 2>&1 | head -30
```
Expected: error 없음

- [ ] **Step 3: 커밋**

```
git add src-tauri/src/main.rs
git commit -m "feat: list_files → load_cache + start_scan 분리"
```

---

### Task 4: app.js — 캐시 로드 + 프로그레스바 + 새로고침 버튼

**Files:**
- Modify: `static/app.js`
- Modify: `static/index.html` (프로그레스바 UI 요소)

**Interfaces:**
- Consumes: `load_cache`, `start_scan` Tauri commands
- Consumes: `scan-progress`, `scan-done` Tauri events

- [ ] **Step 1: index.html에 프로그레스바 요소 추가**

`static/index.html`에서 `id="msg"` 요소를 찾아 아래로 교체 (없으면 달력 위에 추가):

```html
<div id="msg"></div>
<div id="progress-wrap" style="display:none; padding: 8px 16px;">
  <div style="font-size:13px; color:#555; margin-bottom:4px;" id="progress-label">스캔 중...</div>
  <div style="background:#e0e0e0; border-radius:4px; height:6px; width:100%;">
    <div id="progress-bar" style="background:#1E6FE8; height:6px; border-radius:4px; width:0%; transition:width 0.2s;"></div>
  </div>
</div>
```

- [ ] **Step 2: app.js — Tauri 이벤트 리스너 등록 및 load() 재구성**

`app.js` 상단 `const invoke = ...` 바로 아래에 추가:

```js
const listen = window.__TAURI__.event.listen;
```

기존 `load()` 함수 전체를 아래로 교체:

```js
async function load() {
  const msg = document.getElementById("msg");
  try { memos = await invoke("get_memos"); } catch (_) { memos = {}; }
  try { scanPaths = (await invoke("get_config")).scan_paths || []; } catch (_) { scanPaths = []; }

  if (scanPaths.length === 0) {
    msg.textContent = "";
    data = {};
    renderAll();
    openPanel();
    return;
  }

  // 캐시 먼저 시도
  const cached = await invoke("load_cache");
  if (cached) {
    data = cached.files;
    msg.textContent = "";
    animateNext = true;
    renderAll();
    return;
  }

  // 캐시 없으면 스캔 시작
  await startScan();
}

async function startScan() {
  const msg = document.getElementById("msg");
  const wrap = document.getElementById("progress-wrap");
  const bar = document.getElementById("progress-bar");
  const label = document.getElementById("progress-label");

  msg.textContent = "";
  wrap.style.display = "block";
  bar.style.width = "0%";
  label.textContent = "스캔 준비 중...";

  const unlistenProgress = await listen("scan-progress", (e) => {
    const { drive, scanned, total, matched } = e.payload;
    const pct = total > 0 ? Math.min(99, Math.round(scanned / total * 100)) : 0;
    bar.style.width = pct + "%";
    label.textContent = `${drive} 스캔 중... ${scanned.toLocaleString()} / ${total.toLocaleString()} (찾은 파일: ${matched})`;
  });

  const unlistenDone = await listen("scan-done", (e) => {
    unlistenProgress();
    unlistenDone();
    wrap.style.display = "none";
    bar.style.width = "0%";
    const result = e.payload;
    data = result.files || {};
    if (result.skipped && result.skipped.length > 0) {
      msg.textContent = "건너뛴 드라이브: " + result.skipped.join(", ");
    } else {
      msg.textContent = "";
    }
    animateNext = true;
    renderAll();
  });

  await invoke("start_scan");
}
```

- [ ] **Step 3: 사이드바에 새로고침 버튼 추가**

`app.js`에서 `openPanel` 함수 근처(또는 사이드바 렌더 영역)에 `renderSidebar` 또는 해당 버튼 초기화 로직 추가.

`index.html`에서 스캔 폴더 "관리" 버튼 옆에 새로고침 버튼 추가:

```html
<button onclick="startScan()" style="font-size:12px; padding:2px 8px; cursor:pointer;">↺ 새로고침</button>
```

- [ ] **Step 4: 설정 저장 후 재스캔**

`app.js`에서 `savePaths()` 함수 내 저장 성공 후 `startScan()` 호출:

```js
async function savePaths() {
  try {
    await invoke("save_config", { scanPaths: editPaths, extensions: editExts });
  } catch (e) {
    document.getElementById("panelMsg").textContent = e;
    return;
  }
  scanPaths = editPaths.slice();
  closePanel();
  await startScan();   // ← 추가
}
```

- [ ] **Step 5: cargo tauri dev로 동작 확인**

관리자 터미널에서:
```
cargo tauri dev
```

확인 항목:
1. 캐시 없을 때 → 프로그레스바 표시되며 스캔 진행
2. 스캔 완료 → 달력에 파일 표시
3. 앱 재시작 → 프로그레스바 없이 즉시 달력 표시
4. 새로고침 버튼 → 재스캔 및 프로그레스바 표시
5. 설정에서 폴더 변경 후 저장 → 재스캔

- [ ] **Step 6: 커밋**

```
git add static/index.html static/app.js
git commit -m "feat: 캐시 로드 + 프로그레스바 + 새로고침 버튼"
```
