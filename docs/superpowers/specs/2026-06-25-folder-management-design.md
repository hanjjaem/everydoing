# 스캔 폴더 UI 관리 — 설계 문서

작성일: 2026-06-25
선행: [업무정리캘린더 기본 설계](2026-06-25-work-calendar-design.md)

## 목적

`config.json`을 손으로 고치지 않고, 캘린더 웹 화면에서 스캔 대상 폴더를
추가/삭제할 수 있게 한다. 폴더 추가는 윈도우 탐색기 폴더 선택창으로 한다.

## 동작 흐름

1. 캘린더 헤더에 **"폴더 설정"** 버튼 → 누르면 설정 패널이 열린다.
2. 패널에 현재 스캔 폴더 목록 + 각 항목 옆 **삭제(×)** 버튼.
3. **"폴더 추가"** 버튼 → 서버가 윈도우 폴더 선택 다이얼로그를 띄움 →
   마우스로 고른 폴더가 목록에 추가된다.
4. **"저장"** → `config.json`의 `scan_paths`를 갱신하고, 파일 목록을 다시 불러와
   캘린더를 갱신한다.

## 컴포넌트

### 서버 API (`server.py`)
- `GET /api/config` → `{"scan_paths": [...]}` 반환
- `POST /api/config` (body `{"scan_paths": [...]}`) → config.json에 저장, `{"ok": true}`
- `POST /api/pick-folder` → 폴더 선택 다이얼로그를 띄우고 `{"path": "고른경로"}` 반환.
  취소 시 `{"path": ""}`

### 폴더 선택 다이얼로그
파이썬 표준 `tkinter`의 `filedialog.askdirectory`를 쓴다. 웹서버 스레드에서
tkinter를 직접 띄우면 불안정하므로, 다이얼로그만 띄우고 선택 경로를 stdout으로
출력하는 짧은 헬퍼 스크립트(`pick_folder.py`)를 `subprocess`로 실행해 경로를
받아온다. 외부 패키지 없음(표준 라이브러리만).

### 설정 저장 로직 (`everything_client.py`)
순수 함수로 분리해 단위 테스트한다.
- `save_config(config: dict, path="config.json") -> None` — config를 JSON으로 저장.
- `add_scan_path(scan_paths: list[str], new_path: str) -> list[str]` — 경로 추가,
  이미 있으면 그대로(중복 제거). 빈 문자열은 무시.
- `remove_scan_path(scan_paths: list[str], target: str) -> list[str]` — 경로 제거.

`POST /api/config`는 받은 `scan_paths`로 기존 config의 해당 키만 교체해 저장한다
(다른 키 everything_port·extensions·server_port는 보존).

## UI (`static/index.html`, `static/app.js`)
- 헤더에 "폴더 설정" 버튼.
- 설정 패널(처음엔 숨김): 현재 폴더 목록(각 줄에 경로 + × 삭제), "폴더 추가" 버튼,
  "저장" 버튼, "닫기" 버튼.
- "폴더 추가" → `POST /api/pick-folder` → 경로 비어있지 않으면 목록에 추가(중복 시 무시).
- "저장" → `POST /api/config` → 성공 시 패널 닫고 `load()` 재실행으로 캘린더 갱신.

## 에러 처리
- 다이얼로그 취소 → 빈 경로 반환, 목록 변화 없음.
- 중복 경로 → `add_scan_path`가 한 번만 유지.
- 저장 실패(쓰기 오류) → 503/500 + `{"error": ...}`, UI에 메시지 표시.
- 다이얼로그 실행 실패(tkinter 없음 등) → `{"error": ...}` 반환, UI에 메시지.

## 검증
- 단위 테스트: `add_scan_path`(추가·중복·빈값), `remove_scan_path`(제거·없는 값),
  `save_config`+`load_config` 왕복(임시 파일).
- 수동: "폴더 추가"로 다이얼로그가 뜨고 선택 경로가 목록에 추가되는가,
  저장 후 캘린더에 새 폴더 파일이 잡히는가, 삭제 후 사라지는가.

## 범위 밖 (YAGNI)
- 확장자 UI 편집 — 이번엔 폴더만. (확장자는 계속 config.json 직접 수정)
- 경로 직접 타이핑 입력 — 다이얼로그만 제공.
- 폴더별 on/off 토글 — 추가/삭제만.
