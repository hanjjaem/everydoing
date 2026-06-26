# CLAUDE.md

이 저장소에서 작업할 때의 프로젝트 고유 사실·규칙. 전역 규칙은 자동 적용되므로
여기엔 이 프로젝트만의 것을 적는다.

## 프로젝트 개요
내가 진행한 업무 문서를 수정일 기준으로 월 달력에 표시하는 개인용 데스크톱 앱.
"언제 무엇을 했는지"를 한눈에 본다. 개인용, 내 PC에서만 실행.

## 인프라 / 배포 구조
| 구분 | 내용 |
|------|------|
| 인덱스 엔진 | ntfs-reader 크레이트로 NTFS MFT 직접 순회 (Everything 불필요) |
| 백엔드 | Tauri 2 Rust 백엔드 (`src-tauri/src/`) |
| 프런트 | 바닐라 HTML/CSS/JS (`static/`) |
| 실행 | `cargo tauri dev` (관리자 터미널) → 데스크톱 창 |
| 배포 | `cargo tauri build` → NSIS 설치본 (UAC 승격 포함) |
| 플랫폼 | Windows 전용 (NTFS MFT 원시 접근) |
| 설정 파일 | `%APPDATA%\EveryDoing\config.json` (앱이 관리) |

## 프로젝트 결정사항 (왜 이렇게 되어 있는지)
- MFT 직접 인덱싱 (ntfs-reader) — Everything 의존 제거, .exe 단독 실행.
- 시작 시 전체 한 번 스캔 + "새로고침" 재스캔. USN journal 실시간 감시 없음.
- 기준 날짜는 파일 수정일. ntfs-reader의 `OffsetDateTime.unix_timestamp()`를 로컬 날짜로 변환.
- scan_paths에서 드라이브 문자를 자동 추출해 해당 볼륨만 인덱싱.
- 비NTFS / 권한 실패 드라이브는 skipped 목록에 기록하고 계속 진행.
- 설정은 `%APPDATA%\EveryDoing\config.json` (scan_paths·extensions).
- 달력 칸 = "다이어리 칸": 날짜 + 우상단 파일 수(클릭 시 우측 드로어로 파일 목록)
  + 확장자 종류칩(브랜드색 원+약어, 종류별 1개) + 본문 한 줄 메모(칸 여백 클릭 시 인라인 편집).
  활동량을 히트맵 전체 칠로 표현하지 않음(메모 가독성·"경고" 색감 때문에 폐기).
- 메모는 `%APPDATA%\EveryDoing\memos.json`에 날짜별 한 줄로 로컬 저장.
  외부 AI/네트워크 없이 사용자가 직접 입력(프라이버시·로컬 전용 원칙 유지).
- 레거시 Python 코드(Everything 시절)는 `legacy/`에 보관.

## 자주 막히는 것들 (시행착오 메모)
- MFT 접근에는 관리자 권한 필요. `cargo tauri dev`도 관리자 터미널에서 실행.
- 비NTFS 드라이브(USB FAT32 등)는 자동으로 skipped 처리되고 UI에 안내 표시.

## 코드 수정 후 체크리스트
- `cargo test` (src-tauri/ 디렉토리에서) 통과 확인
- 관리자 터미널에서 `cargo tauri dev` 후 달력에 파일 뜨는지 확인

## 환경변수
<!-- 없음 -->
