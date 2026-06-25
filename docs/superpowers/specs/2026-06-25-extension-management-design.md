# 파일 유형(확장자) UI 선택 — 설계 문서

작성일: 2026-06-25
선행: [스캔 폴더 UI 관리](2026-06-25-folder-management-design.md)

## 목적

설정 패널에서 스캔 대상 파일 유형(확장자)을 체크박스로 켜고 끄고, 목록에 없는
확장자는 직접 추가할 수 있게 한다. 저장하면 `config.json`의 `extensions`가 갱신되고
캘린더가 다시 로드된다.

## UI (설정 패널에 영역 추가)

폴더 목록 아래에 **"파일 유형"** 영역을 둔다.
- **프리셋 체크박스** — hwp, hwpx, doc, docx, xls, xlsx, ppt, pptx, pdf 를 체크박스로
  나열. 현재 config의 `extensions`에 있는 것은 체크된 상태로 표시.
- **현재 config에 있으나 프리셋에 없는 확장자**(사용자가 전에 추가한 것)도 체크박스로
  표시하고 체크해 둔다. 즉 체크박스 목록 = 프리셋 ∪ 현재 extensions.
- **직접 추가** — 입력칸 + "추가" 버튼. 임의 확장자를 넣으면 정규화 후 체크박스 목록에
  새 항목으로 추가되고 체크된다(이미 있으면 무시).
- **저장**은 폴더와 공용 버튼. 누르면 체크된 확장자 전체 + 폴더 목록을 함께 저장하고
  패널을 닫은 뒤 캘린더를 갱신한다.

## 입력 정규화 (`everything_client.py`)

순수 함수로 분리해 단위 테스트한다.
- `normalize_extension(raw: str) -> str | None` — 앞뒤 공백 제거, 맨 앞 점(.) 제거,
  소문자화. 결과가 빈 문자열이면 None.
- `normalize_extensions(raws: "list") -> "list[str]"` — 각 항목을 normalize_extension으로
  변환하고, None은 버리고, 순서를 유지한 채 중복을 제거한 리스트 반환.

## 서버 (`server.py`)

- `GET /api/config` → 응답에 `extensions`도 포함:
  `{"scan_paths": [...], "extensions": [...]}`.
- `POST /api/config` → 기존 scan_paths 처리에 더해, body에 `extensions`가 있으면
  list인지 검증(아니면 400) 후 `normalize_extensions`로 정규화해 config의 `extensions`
  키를 교체. body에 extensions가 없으면 기존 값을 보존. 나머지 키도 보존.

## 데이터 흐름

1. "폴더 설정" 클릭 → `GET /api/config` → scan_paths와 extensions 둘 다 받음.
2. 패널: 폴더 목록 + 확장자 체크박스(프리셋 ∪ 현재, 현재는 체크).
3. "추가" → 입력 정규화 → 체크박스 목록에 추가(중복 무시).
4. "저장" → `POST /api/config` body `{scan_paths, extensions}` → 저장 →
   패널 닫고 `load()`로 캘린더 갱신.

## 에러 처리

- `extensions`가 list가 아니면 400 + `{"error": ...}`.
- 정규화 후 빈 리스트면 그대로 저장(사용자 선택 — 결과는 빈 캘린더).
- 저장 실패(OSError) → 500. UI에 메시지.

## 검증

- 단위 테스트: `normalize_extension`(점·대문자·공백·빈값), `normalize_extensions`
  (중복 제거·순서 유지·None 제거).
- 수동: 체크박스 토글·직접 추가·저장 후 캘린더가 해당 유형만 잡는지.

## 범위 밖 (YAGNI)

- 확장자별 색상/아이콘, 카테고리 그룹핑.
- 확장자 삭제 UI(체크 해제로 끄면 충분 — 프리셋은 항상 보이고, 커스텀은 체크 해제 시
  저장에서 빠진다. 다음 패널 열기 때 config에 없으면 사라짐).
