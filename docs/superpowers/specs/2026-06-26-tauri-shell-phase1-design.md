# EveryDoing 데스크톱 전환 — 1단계: Tauri 셸 설계

날짜: 2026-06-26

## Context

이 앱을 기관에 `.exe`로 배포하기 위해 데스크톱 앱으로 전환한다. 최종 목표는
NTFS MFT를 직접 읽어 Everything 의존까지 제거한 독립 실행 앱(2단계)이다.

전환 위험을 분산하기 위해 **2단계로 분해**한다:
- **1단계(이 문서): Tauri 셸** — 웹앱을 진짜 데스크톱 창으로 띄우고, Python
  백엔드(HTTP 서버)를 Rust 커맨드로 이전. 파일 목록은 1단계에선 **Everything을
  그대로 호출**(인덱서는 2단계). Tauri·빌드·프런트 연동이라는 "배관"을 MFT 난제와
  분리해 먼저 안정화한다.
- **2단계(별도 스펙): MFT 직접 인덱서** — Everything 호출을 네이티브 NTFS MFT
  읽기로 교체. Everything 졸업.

스택 선택 근거: Windows 전용 + 이미 완성된 웹 UI + 작은 백엔드 → Tauri가 최적
(WebView2 OS 내장으로 exe ~10MB, 웹 UI 그대로 재사용, 백엔드가 작아 Rust 이전 부담
적음). 개발 PC에 Rust 1.95·Node 24 설치 확인됨.

## 1단계 범위

웹앱(`server.py` + `everything_client.py` + `static/`)을 Tauri 데스크톱 앱으로
전환한다. 기능 동등(feature parity)이 목표 — 새 기능 없음.

## 프로젝트 구조

```
업무정리캘린더/
  src-tauri/
    Cargo.toml          # tauri, serde, serde_json, reqwest(blocking), chrono, dirs, opener
    tauri.conf.json     # 창 설정, frontendDist = ../static, 번들 타깃
    build.rs
    icons/
    src/
      main.rs           # 커맨드 등록 + 창 부트스트랩
      everything.rs     # Everything HTTP 쿼리 + FILETIME→로컬날짜 그룹화
      store.rs          # config.json / memos.json 읽기·쓰기 (APPDATA)
  static/               # index.html 그대로, app.js만 invoke로 수정
  server.py 등          # 1단계에선 잔존(미사용), 2단계 완료 후 제거
```

## Rust 커맨드 (기존 API 1:1 대응)

| 커맨드 | 시그니처 | 동작 | 이식 출처 |
|---|---|---|---|
| `list_files` | `() -> Result<Map<String, Vec<FileItem>>, String>` | Everything HTTP 쿼리, 날짜별 그룹 | `everything_client.fetch_files`/`group_by_date` |
| `open_path` | `(path: String, mode: String) -> Result<(), String>` | `mode=="folder"`→탐색기 선택, else 기본앱 | `server._handle_open` |
| `pick_folder` | `() -> Result<Option<String>, String>` | 네이티브 폴더 선택(취소=None) | `pick_folder.py` |
| `get_config` | `() -> Result<ConfigOut, String>` | scan_paths·extensions 반환 | `server` GET /api/config |
| `save_config` | `(scan_paths: Vec<String>, extensions: Vec<String>) -> Result<(), String>` | 검증 후 저장 | `server._handle_save_config` |
| `get_memos` | `() -> Result<Map<String,String>, String>` | memos.json (없으면 `{}`) | `everything_client.load_memos` |
| `save_memo` | `(date: String, text: String) -> Result<(), String>` | 빈 텍스트면 키 삭제 후 저장 | `everything_client.set_memo`/`save_memos` |

`FileItem { name: String, path: String }`. `ConfigOut { scan_paths, extensions }`.

### 이식 시 보존할 로직 (기존 동작 동일성)
- **검색 쿼리**: `ext:hwp;docx ("C:\\path\\" | "D:\\path\\")` 형식
  (`build_search_query` — 경로 끝 백슬래시 정규화 포함).
- **FILETIME→날짜**: 1601 기준 100ns → 로컬 타임존 `YYYY-MM-DD`. 0·파싱불가는 제외.
- **날짜 그룹화**: 변환 실패분은 `"unknown"` 버킷.
- **확장자 정규화**: 앞 `.` 제거, 소문자, 중복 제거·순서 보존, 빈 값 드롭.
- **config 검증**: `scan_paths` 비면 거부, `extensions` 정규화 후 비면 거부.

## 데이터 파일 위치 (변경)

설치형 앱은 작업폴더 쓰기가 막힐 수 있어, 데이터를 사용자 앱데이터로 옮긴다.
- 디렉터리: `%APPDATA%\EveryDoing\` (`dirs::config_dir()` 하위)
- `config.json` — 최초 실행 시 기본값으로 생성:
  `everything_host="localhost"`, `everything_port=8080`,
  `scan_paths=[]`, `extensions=["hwp","hwpx","doc","docx","xls","xlsx","ppt","pptx","pdf"]`.
- `memos.json` — 없으면 `{}`.
- 기존 repo 루트의 `config.json`/`memos.json`은 더 이상 런타임에 쓰지 않음
  (개발 참고용으로 남김, gitignore 유지).

## 프런트 변경 (`static/app.js`)

`fetch("/api/...")` 호출을 Tauri `invoke`로 교체. 렌더링·UI 로직은 불변.
- `/api/files` GET → `invoke("list_files")`
- `/api/open` POST → `invoke("open_path", { path, mode })`
- `/api/pick-folder` POST → `invoke("pick_folder")`
- `/api/config` GET → `invoke("get_config")`
- `/api/config` POST → `invoke("save_config", { scanPaths, extensions })`
- `/api/memos` GET → `invoke("get_memos")`
- `/api/memos` POST → `invoke("save_memo", { date, text })`
- 경로 복사: `navigator.clipboard.writeText` → Tauri clipboard 플러그인.
- `window focus 시 load()` 재조회는 Tauri 창 focus 이벤트로 동일 동작.
- 에러 메시지: Everything 미연결 시 기존 안내 문구 유지(1단계는 아직 Everything 의존).

`invoke`는 `index.html`에 Tauri API 스크립트 추가로 노출(`window.__TAURI__`).

## 창 최적화 (`static/index.html`)

- 가짜 "회색 여백 + 윈도우 카드"(`body` 그라데이션 패딩, `.app` border-radius/shadow)
  제거 → `.app`이 네이티브 창을 꽉 채움(`100vw/100vh`, 라운드·그림자 없음).
- 타이틀바는 OS 기본(데코레이션 유지). 좌상단 `EveryDoing` 브랜드는 앱바에 유지.

## 빌드 · 배포

- 개발: `cargo tauri dev` (창 실행, 핫리로드).
- 배포: `cargo tauri build` → `src-tauri/target/release/bundle/`에 설치파일(NSIS/MSI).
- 아이콘: 임시 기본 아이콘으로 시작(브랜드 아이콘은 후속).

## 검증

1. `cargo tauri dev`로 창이 뜨고 달력이 렌더되는지.
2. Everything HTTP 서버 켠 상태에서 `list_files`가 파일을 채우는지(기존과 동일 결과).
3. 메모 입력·저장 후 `%APPDATA%\EveryDoing\memos.json` 생성/갱신, 재시작 후 유지.
4. 설정에서 폴더 추가(네이티브 선택)·확장자 저장 → config.json 반영.
5. 파일 열기·폴더 열기·경로 복사 동작.
6. `cargo tauri build` 성공, 생성된 설치파일로 설치 후 실행 확인.
7. Everything 끈 상태에서 안내 메시지 표시.

## 범위 밖 (2단계 이후)

- MFT 직접 인덱싱 / USN 실시간 반영 / Everything 제거 / 관리자 권한 승격.
- 코드 서명, 자동 업데이트, 브랜드 아이콘.
- `server.py` 등 Python 파일 삭제(2단계 완료 후).
