# CLAUDE.md

이 저장소에서 작업할 때의 프로젝트 고유 사실·규칙. 전역 규칙은 자동 적용되므로
여기엔 이 프로젝트만의 것을 적는다.

## 프로젝트 개요
내가 진행한 업무 문서를 수정일 기준으로 월 달력에 표시하는 개인용 로컬 웹.
"언제 무엇을 했는지"를 한눈에 본다. 개인용, 내 PC에서만 실행.

## 인프라 / 배포 구조
| 구분 | 내용 |
|------|------|
| 인덱스 엔진 | 설치된 Everything의 HTTP 서버를 쿼리 (직접 인덱싱 안 함) |
| 백엔드 | Python 3 표준 라이브러리만 (`server.py`, `everything_client.py`) |
| 프런트 | 바닐라 HTML/CSS/JS (`static/`) |
| 실행 | `python server.py` → 브라우저 `http://localhost:5000` |
| 배포 | 없음. 로컬 전용 |
| 플랫폼 | Windows 전용 (`os.startfile`, `explorer /select`) |

## 프로젝트 결정사항 (왜 이렇게 되어 있는지)
- 인덱싱을 직접 만들지 않고 Everything 재활용 — 전 드라이브 스캔을 직접 walk 하면 느림.
- 시작 시 전체 한 번 로드 후 브라우저 메모리 보관 — 월 이동 시 재요청 없게.
- 기준 날짜는 파일 수정일. Everything의 FILETIME(1601 기준)을 변환해 사용.
- 설정은 `config.json` 한 곳 (경로·포트·확장자). 코드에 하드코딩 금지.

## 자주 막히는 것들 (시행착오 메모)
- Everything → 도구 → 옵션 → HTTP 서버를 켜야 `/api/files`가 동작한다.
  포트가 config.json의 `everything_port`와 일치해야 함.

## 코드 수정 후 체크리스트
- `python -m pytest tests/` 통과 확인
- `python server.py` 재실행 후 브라우저 새로고침으로 동작 확인

## 환경변수
<!-- 없음 -->
