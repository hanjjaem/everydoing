# 캐시 + 프로그레스바 설계

**날짜:** 2026-06-29  
**대상 브랜치:** feat/ui-refresh

## 목표

매 실행마다 MFT 전체 재스캔(30~45초) 문제를 해결한다.  
첫 스캔 이후엔 캐시를 즉시 로드해 달력이 바로 뜨고, 스캔 중엔 프로그레스바로 진행 상황을 보여준다.

---

## 아키텍처

### Rust 변경

#### 1. `store.rs` — 캐시 저장/로드

```
%APPDATA%\EveryDoing\index.json
```

저장 형식:
```json
{
  "scan_paths": ["D:\\업무"],
  "extensions": ["hwp", "pdf"],
  "files": { "2026-06-29": [{ "name": "...", "path": "..." }] },
  "skipped": []
}
```

- `save_index(cfg: &Config, result: &ScanResult)` — 스캔 완료 후 저장
- `load_index(cfg: &Config) -> Option<ScanResult>` — 캐시 로드. scan_paths·extensions가 config와 다르면 `None` 반환 (무효화)

#### 2. `main.rs` — 커맨드 분리

| 커맨드 | 역할 |
|--------|------|
| `load_cache()` | 캐시 로드. 있으면 `ScanResult`, 없으면 `null` |
| `start_scan()` | 백그라운드 스캔 시작. 진행률 이벤트 emit |

기존 `list_files()` 제거.

#### 3. `indexer.rs` — 진행률 emit

`scan()` 시그니처 변경:
```rust
pub fn scan(cfg: &Config, app: AppHandle) -> ScanResult
```

드라이브별 MFT 순회 중 매 10,000개 파일마다:
```rust
app.emit("scan-progress", ScanProgress {
    drive: "D:",
    scanned: 50000,
    total: 837000,   // Mft::file_count() 로 미리 파악
    matched: 12,
})
```

완료 시:
```rust
app.emit("scan-done", result)
```

### 프론트 변경 (`app.js`)

#### 시작 플로우

```
앱 시작
  → load_cache() 호출
  → 캐시 있음: 달력 즉시 표시 (스캔 없음)
  → 캐시 없음 or scan_paths 없음: 설정 패널 오픈 대기
```

설정 저장 후 → `start_scan()` 호출 → 프로그레스바 표시

#### 새로고침 버튼

사이드바에 "새로고침" 버튼 추가.  
클릭 → `start_scan()` → 프로그레스바 → 완료 시 달력 갱신.

#### 프로그레스바 UI

달력 위 영역에 표시:
```
[스캔 중] D: ████████░░ 50,000 / 837,000
```
- 드라이브명 + 진행 수치 + 바
- `scan-done` 이벤트 수신 시 제거

---

## 캐시 무효화 기준

`load_index()`에서 저장된 scan_paths·extensions와 현재 config를 비교.  
다르면 `None` 반환 → 재스캔.

---

## 에러 처리

- 캐시 파일 읽기 실패 → 무시하고 재스캔
- 캐시 파일 쓰기 실패 → 무시 (스캔 결과는 정상 표시)
- 스캔 중 에러 → `scan-done`에 skipped 목록 포함

---

## 변경 범위

| 파일 | 변경 |
|------|------|
| `src-tauri/src/store.rs` | `save_index`, `load_index` 추가 |
| `src-tauri/src/indexer.rs` | `scan()` 시그니처에 `AppHandle` 추가, 진행률 emit |
| `src-tauri/src/main.rs` | `list_files` → `load_cache` + `start_scan` 분리 |
| `static/app.js` | 시작 플로우 재구성, 프로그레스바, 새로고침 버튼 |
