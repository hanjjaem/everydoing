# EveryDoing 데스크톱 전환 — 2단계: MFT 직접 인덱서 설계

날짜: 2026-06-26

## Context

1단계(Tauri 셸)로 웹앱을 데스크톱 앱으로 전환하고, 파일 목록은 Everything HTTP를
임시로 호출했다. 2단계는 그 임시 의존을 제거한다 — Everything이 하던 일(NTFS MFT를
직접 읽어 파일 색인)을 앱이 자체적으로 수행해, **Everything 없이 .exe만으로 도는
독립 실행 앱**을 완성한다(최종 목표).

브레인스토밍 결정사항:
- **갱신 주기**: 앱 시작 시 1회 스캔 + "새로고침"으로 재스캔. USN journal 실시간
  감시는 하지 않음(용도상 분 단위 실시간성 불필요, 작업 단순화).
- **권한**: 항상 관리자 승격(UAC). MFT 원시 볼륨 접근에 필수.
- **드라이브 범위**: `scan_paths`에 등장하는 드라이브만 자동 추출해 인덱싱(전체
  볼륨 읽기 후 경로로 필터). 사용자는 기존처럼 폴더만 고름.
- **비NTFS/읽기 실패 드라이브**: 그 경로는 건너뛰고 작은 안내.

## 구현 방식

`ntfs-reader` 크레이트로 볼륨의 모든 파일을 순회한다. 이 크레이트는 볼륨 MFT를 읽어
`FileInfo`(이름·전체 경로·타임스탬프)를 복원해주므로, MFT 바이트를 직접 파싱하지
않는다. 대안(`mft` 원시 덤프 파서, ColinFinck `ntfs` 저수준)보다 우리 용도("드라이브
순회 → 경로·수정일 추출")에 직접적이다.

## 동작 흐름

```
앱 시작(관리자 승격)
  → load_config()의 scan_paths에서 드라이브 문자 추출 (예: C:, D:)
  → 각 NTFS 볼륨을 ntfs-reader로 순회
     · 디렉터리 제외, 확장자가 config.extensions에 속하는 파일만
     · 전체 경로가 scan_paths 중 하나로 시작하는 파일만(prefix 필터)
     · {name, full_path, modified(FILETIME)} 수집
  → group_by_date 로 날짜별 그룹화 → 프런트 반환
"새로고침"(또는 창 focus) 시 재스캔
```

## 변경 파일

### 신규: `src-tauri/src/indexer.rs`
- `fn drives_from_paths(scan_paths: &[String]) -> Vec<String>` — 경로들에서 고유
  드라이브 문자 추출(예: `["C:", "D:"]`). 순수 함수.
- `fn path_matches(full_path: &str, scan_paths: &[String]) -> bool` — 전체 경로가
  scan_paths 중 하나의 prefix로 시작하는지(대소문자 무시, 폴더 경계 고려). 순수 함수.
- `fn ext_matches(name: &str, extensions: &[String]) -> bool` — 파일명 확장자가
  허용 목록에 있는지. 순수 함수.
- `struct ScanResult { files: BTreeMap<String, Vec<FileItem>>, skipped: Vec<String> }`
  — 그룹화된 결과 + 건너뛴 드라이브(비NTFS/실패) 목록.
- `fn scan(cfg: &store::Config) -> ScanResult` — 드라이브 순회·필터·그룹화 종합.
  볼륨 열기 실패 시 그 드라이브를 `skipped`에 추가하고 계속.

### 재사용: `everything.rs`의 `FileItem`·`filetime_to_date`·`group_by_date`
- 이들을 `indexer.rs`로 이동(또는 공용 모듈로). `everything.rs` 자체(쿼리·HTTP fetch)는 제거.

### `src-tauri/src/main.rs`
- `mod everything;` → `mod indexer;`
- `list_files` 커맨드: `everything::fetch_files` → `indexer::scan` 반환 구조로.
  반환 타입을 `{ files, skipped }` 형태로 바꿔 프런트가 건너뛴 드라이브를 안내하게.

### `src-tauri/src/store.rs`
- `Config`에서 `everything_host`·`everything_port` 제거. 기본 config도 갱신.
  (역호환: 기존 config.json에 남은 두 필드는 무시되도록 `#[serde(default)]`/미사용 처리.)

### `src-tauri/tauri.conf.json` + NSIS
- 실행 파일 manifest에 `requireAdministrator` 설정(UAC 항상 승격).

### `static/app.js` / `index.html`
- `list_files` 응답 구조 변경 대응: `data = result.files`, `skipped` 있으면 `#msg`에
  "○ 드라이브는 NTFS가 아니거나 읽을 수 없어 건너뜀" 안내.
- 로딩 상태: 스캔 중 "인덱싱 중..." 표시(시작·새로고침 시).
- Everything 관련 안내 문구 제거.

### 정리(Everything 졸업)
- `everything.rs` 삭제. repo 루트의 `server.py`·`everything_client.py`·`pick_folder.py`·
  `config.json`(8080 포함)·`tests/`(Python) 제거 또는 `legacy/`로 이동.
  CLAUDE.md의 "Everything 재활용" 기술을 "MFT 직접 인덱싱"으로 갱신.

## 에러 처리

- 볼륨 열기 실패(비NTFS·권한·부재): 그 드라이브 건너뛰고 `skipped`에 기록, 나머지 진행.
- 관리자 권한 미승격으로 모든 볼륨 실패: 프런트에 "관리자 권한이 필요합니다" 안내.
- scan_paths 비어있음: 빈 결과 + "설정에서 폴더를 추가하세요"(기존 빈 상태 UI 재사용).

## 테스트

순수 함수는 `cargo test`로 TDD:
- `drives_from_paths`: 다중 경로에서 드라이브 중복 제거, 대소문자.
- `path_matches`: prefix 매칭, 대소문자 무시, 폴더 경계(`C:\Doc`가 `C:\Document`에
  오매칭 안 되게).
- `ext_matches`: 확장자 대소문자 무시, 미허용 제외, 확장자 없는 파일 제외.
- `group_by_date`(이식분): 기존 동작 유지.

볼륨 순회(`scan`)는 실제 실행(관리자)로 수동 검증 — 단위 테스트는 실디스크 의존이라 제외.

## 검증 (수동, 관리자 실행)

1. `cargo tauri dev`(관리자) → UAC 승격 후 창 실행.
2. scan_paths의 드라이브 파일들이 달력에 뜨는지(Everything 없이).
3. 새 파일 생성 후 "새로고침" → 반영.
4. 비NTFS 드라이브를 scan_path에 넣으면 건너뛰고 안내.
5. `cargo tauri build` → 설치 → 시작 시 UAC 뜨고 정상 동작.
6. Everything 완전히 끈 상태에서도 동일 동작(의존 제거 확인).

## 범위 밖

- USN journal 실시간 갱신.
- 비NTFS 볼륨 지원(FAT/exFAT/네트워크).
- 코드 서명, 자동 업데이트.
