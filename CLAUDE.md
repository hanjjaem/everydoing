# CLAUDE.md

이 저장소에서 작업할 때의 프로젝트 고유 사실·규칙. 전역 규칙은 자동 적용되므로
여기엔 이 프로젝트만의 것을 적는다.

## 프로젝트 개요
내가 진행한 업무 문서를 수정일 기준으로 월 달력에 표시하는 개인용 데스크톱 앱.
"언제 무엇을 했는지"를 한눈에 본다. 개인용, 내 PC에서만 실행.

## 인프라 / 배포 구조
| 구분 | 내용 |
|------|------|
| 인덱스 엔진 | 드라이브 전체는 ntfs-reader로 NTFS MFT 직접 순회, 일반 폴더는 walkdir 순회 (Everything 불필요) |
| 백엔드 | Tauri 2 Rust 백엔드 (`src-tauri/src/`) |
| 프런트 | 바닐라 HTML/CSS/JS (`static/`) |
| 실행 | `cargo tauri dev` (관리자 터미널) → 데스크톱 창 |
| 배포 | `cargo tauri build` → NSIS 설치본 (UAC 승격 포함) |
| 플랫폼 | Windows 전용 (NTFS MFT 원시 접근) |
| 설정 파일 | `%APPDATA%\EveryDoing\config.json` (앱이 관리) |

## 프로젝트 결정사항 (왜 이렇게 되어 있는지)
- MFT 직접 인덱싱 (ntfs-reader) — Everything 의존 제거, .exe 단독 실행.
- 시작 시 저장된 색인(index.json)을 먼저 보여 주고, 뒤에서 전체를 다시 스캔해 반영(0.2.0~).
  켜 둔 동안의 변경은 ↺ 수동 재스캔. USN journal 실시간 감시 없음.
- 기준 날짜는 파일 수정일. ntfs-reader의 `OffsetDateTime.unix_timestamp()`를 로컬 날짜로 변환.
  수정 시각(초)은 `FileItem.modified`로 남겨 하루 안에서 최근 순 정렬·시각 표시에 쓴다.
- 스캔 경로 분기는 `indexer::scan_all` 한 곳: 드라이브 전체(`D:\`)는 MFT, 일반 폴더는 walkdir.
  드라이브 전체에 포함된 폴더는 다시 읽지 않는다. `main.rs`와 `bin/dev_web.rs`가 같이 쓴다.
- 제외 규칙은 `indexer::is_noise`: 휴지통·시스템 폴더(드라이브 전체 스캔일 때만), 점(.) 폴더,
  node_modules, Office 임시 파일(`~$`). 스캔 폴더 기준 상대 경로로 판단하므로 사용자가 직접 고른 폴더는 제외되지 않는다.
- 비NTFS / 권한 실패 드라이브는 skipped 목록에 기록하고 계속 진행.
- 색인 저장 조건: 하위 폴더·파일 일부를 못 읽어도 저장한다. 스캔 폴더 자체를 못 읽은 경우
  (`ScanResult.root_failed`)에만 저장하지 않고 이전 색인을 지킨다.
- 창 포커스 때의 삭제 확인(`prune_deleted`)은 보고 있는 달만, 메인 스레드 밖에서 한다.
- 설정은 `%APPDATA%\EveryDoing\config.json` (scan_paths·extensions).
- 달력 칸(현재 구현): 날짜 + 파일 2개 미리보기("+N 더보기", 클릭 시 우측 드로어) + AI 요약 라벨 + 한 줄 메모.
  칸 재구성 방향은 `docs/개선-계획.md` B1·B2 참고(결정 전).
  활동량을 히트맵 전체 칠로 표현하지 않음(메모 가독성·"경고" 색감 때문에 폐기).
- 월 이동은 달력 카드 위의 "월 인덱스" 한 줄(연도 · 월 탭 12개 · 오늘)로만 한다. 탭의 점은 기록이 있는 달.
  탭 은유는 여기 한 곳에만 쓴다(제목 표시줄은 일반 제목). 사이드바에는 월 이동을 두지 않는다.
- 검색은 전체 기간 대상(파일명·폴더 경로). 검색어가 있으면 달력 대신 결과 목록을 보여 준다.
- 글꼴(Pretendard)은 `static/fonts/`에 포함한다. CDN 등 외부 리소스를 화면에서 불러오지 않는다
  ("기본 상태에서 외부 통신 없음" 약속).
- 메모는 `%APPDATA%\EveryDoing\memos.json`에 날짜별 한 줄로 로컬 저장.
  외부 AI/네트워크 없이 사용자가 직접 입력(프라이버시·로컬 전용 원칙 유지).
- AI 요약(OpenAI)은 **선택 기능이며 기본 꺼짐**. 외부 배포 대비 전송 안전장치 패턴 적용:
  - opt-in: `Config.ai_summary_enabled`(기본 false). 백엔드 `generate_summaries`가
    꺼져 있으면 전송 자체를 거부 → 끄면 외부 통신 0.
  - BYOK: 사용자가 자기 OpenAI 키 입력, `config.json`에 로컬 저장(서버 미경유).
  - on-demand: 로드/스캔 시 자동 전송하지 않음. 사이드바 'AI 요약 생성' 버튼 +
    매번 confirm 동의 시에만 전송. 전송 대상은 파일명뿐(파일 내용 아님).
  - 설정 패널에 전송 고지 문구 노출.
  - TODO(여력 시): 외부 전송 없는 로컬 요약 경로(예: TextRank류) 추가.
- 레거시 Python 코드(Everything 시절)는 git 이력에만 남김. 2026-11-11 공공 GitLab 보안 검사(Semgrep) 대응으로 `legacy/` 폴더를 저장소에서 제거함(제거 직전 커밋 3ade0f4에서 확인 가능).

## 자주 막히는 것들 (시행착오 메모)
- MFT 접근에는 관리자 권한 필요. `cargo tauri dev`도 관리자 터미널에서 실행.
- 비NTFS 드라이브(USB FAT32 등)는 자동으로 skipped 처리되고 UI에 안내 표시.
- `ntfs_reader`의 `info.path`는 `\\.\D:\폴더\파일` 형태로 반환됨. `path_matches()` 전에 반드시 `strip_prefix("\\\\.\\")` 로 정규화 필요.
- 달력 격자 열은 `repeat(7, minmax(0, 1fr))`이어야 한다. `1fr`만 쓰면 긴 파일명이 열 너비를 밀어낸다.
- 긴 경로를 왼쪽에서 줄이려고 `direction: rtl`을 쓰는 곳은 글자를 `<bdi>`로 감싼다(안 그러면 `D:\`가 `\:D`로 보임).
- 설치 폴더의 실행 파일이 최신 소스와 다를 수 있다. 화면을 확인할 때는
  `target\debug\everydoing.exe --preview-data-dir <절대경로>`로 사용자 데이터와 분리해 띄운다(디버그 빌드 전용).

## 코드 수정 후 체크리스트
- `cargo test` (src-tauri/ 디렉토리에서) 통과 확인
- `node --test "tests/*.test.cjs"` (저장소 루트에서) 통과 확인
- 관리자 터미널에서 `cargo tauri dev` 후 달력에 파일 뜨는지 확인

## 환경변수
<!-- 없음 -->
