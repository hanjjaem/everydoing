# 가벼운 Apple 테마 + 다크모드 — 설계 문서

작성일: 2026-06-25
참고: Apple 웹 디자인 가이드(VoltAgent/awesome-design-md, apple/DESIGN.md)에서 가벼운 요소만 차용.

## 목적

현재 기본 마크업 수준의 캘린더 UI에 Apple 느낌의 가벼운 테마를 입히고 다크모드를
추가한다. 구조·기능은 그대로 두고 외관만 개선한다. 개인용 로컬 앱이므로 반응형·
풀블리드 같은 대형 레이아웃 장치는 도입하지 않는다.

## 핵심 접근

외관을 CSS 변수(토큰) 한 곳에 모은다. `static/index.html`의 `<style>`을 토큰 기반으로
재작성하고, 구조(HTML 요소)와 동작(app.js)은 거의 그대로 둔다. 다크모드는 토큰 값만
바꿔 전체에 반영한다.

## 1. 디자인 토큰 (`:root`)

Apple DESIGN.md에서 가벼운 것만:
- **색** — 강조색 `#0066cc`(파랑). 배경/표면/테두리/텍스트/흐린텍스트용 그레이스케일.
- **타이포** — 시스템 폰트 스택 `-apple-system, BlinkMacSystemFont, "Malgun Gothic", sans-serif`.
  큰 글자(제목)엔 약간 좁은 자간(letter-spacing: -0.02em).
- **간격** — 8px 기준 리듬: `--space-1:4px --space-2:8px --space-3:16px --space-4:24px`.
- **모서리** — `--radius-sm:8px`, `--radius-pill:9999px`.
- 모든 색·간격·모서리를 `var(--…)`로 정의.

## 2. 컴포넌트 단장 (구조 변경 없음, CSS만)

- **버튼** — 알약형(`--radius-pill`), 강조색 배경/흰 글자. hover 시 약간 밝게,
  press(`:active`) 시 `transform: scale(0.98)`.
- **달력 칸(.cell)** — 카드 느낌: 옅은 테두리 + `--radius-sm`. `.today`는 강조색 2px 링(outline).
- **배지(.badge)** — 알약형, 강조색 배경.
- **파일 링크(.file)** — 강조색 텍스트, hover 시 밑줄.
- **입력칸(#search, #extInput)** — 라운드 테두리, `:focus` 시 강조색 링(box-shadow).
- **설정 패널(#panel)** — 카드: 라운드 + 옅은 그림자(`box-shadow`).
- 7열 그리드·요일 헤더 구조는 유지, 여백·색만 토큰화.

## 3. 다크모드

- `:root` = 라이트 토큰. `:root[data-theme="dark"]` = 다크 토큰(배경 어둡게,
  텍스트 밝게, 강조색은 약간 밝은 파랑 `#2997ff`).
- 헤더에 **토글 버튼**(`id="themeToggle"`, ☀/☾ 표시).
- 클릭 시: `document.documentElement` 의 `data-theme` 속성을 light↔dark 전환,
  `localStorage.setItem("theme", ...)` 저장, 버튼 아이콘 갱신.
- 페이지 로드 시: `localStorage`의 theme를 읽어 적용(없으면 light). 깜빡임 방지를 위해
  가능한 한 이른 시점(인라인 스크립트 또는 app.js 최상단)에 적용.
- 이 토글/복원 로직만 `app.js`에 소량 추가. 나머지는 CSS.

## 4. 변경 파일

- `static/index.html` — `<style>` 토큰 기반 재작성, 헤더에 테마 토글 버튼 추가.
- `static/app.js` — 테마 적용/토글/저장 로직(함수 `applyTheme`, `toggleTheme`) + 버튼 배선 + 로드 시 복원.

## 5. 에러 처리

- `localStorage` 사용 불가(차단 환경) 시 try/catch로 무시하고 기본(light)으로 동작.

## 6. 검증

- 자동 테스트 없음(순수 외관). `node --check static/app.js`, 기존 `python -m pytest tests/`
  18개 회귀 통과 확인.
- 수동: 라이트/다크 토글이 즉시 반영되는가, 새로고침 후 선택이 유지되는가,
  버튼·칸·배지·입력칸·패널이 두 모드 모두에서 읽기 쉽고 깔끔한가, today 강조가 보이는가.

## 범위 밖 (YAGNI)

- 반응형 breakpoint, 풀블리드 섹션, 네비게이션 바.
- SF Pro 웹폰트 다운로드(시스템 폰트로 대체).
- OS 설정 자동 추종(prefers-color-scheme) — 명시적 토글만.
- 애니메이션·마이크로 인터랙션 과다 적용(버튼 press 정도만).
