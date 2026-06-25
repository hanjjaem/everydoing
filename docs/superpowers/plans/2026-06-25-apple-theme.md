# 가벼운 Apple 테마 + 다크모드 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 캘린더 UI에 CSS 변수(토큰) 기반의 가벼운 Apple 테마를 입히고, 토글로 전환·기억되는 다크모드를 추가한다.

**Architecture:** `static/index.html`의 `<style>`을 디자인 토큰(`:root` CSS 변수) 기반으로 재작성하고 컴포넌트(버튼/칸/배지/입력/패널)를 토큰으로 단장한다. 다크모드는 `:root[data-theme="dark"]`에서 토큰 값만 바꾸고, `app.js`가 토글·localStorage 저장·로드시 복원을 담당한다. HTML 구조와 기존 기능 로직은 그대로.

**Tech Stack:** 바닐라 HTML/CSS/JS. (자동 테스트 없음 — 외관; node --check + 기존 pytest 회귀.)

## Global Constraints

- 구조(HTML 요소)와 기능 로직(달력 렌더·검색·설정 패널·파일 열기)은 변경하지 않는다 — CSS와 테마 토글 로직만 추가/변경.
- 강조색 라이트 `#0066cc`, 다크 `#2997ff`. 8px 간격 리듬. 알약형 버튼(`--radius-pill:9999px`), 카드 모서리 `--radius-sm:8px`.
- 폰트는 시스템 스택 `-apple-system, BlinkMacSystemFont, "Malgun Gothic", sans-serif` (웹폰트 다운로드 금지).
- 다크모드는 명시적 토글만(prefers-color-scheme 자동추종 안 함). 선택은 localStorage에 저장, 로드 시 복원, 없으면 light.
- localStorage 접근은 try/catch로 감싸 실패해도 기본 동작 유지.
- 반응형 breakpoint·풀블리드·네비바 도입 안 함(YAGNI).
- 기존 pytest 18개는 영향 없어야 함.

---

## File Structure

- `static/index.html` — `<style>` 전체를 토큰 기반으로 재작성, 헤더에 테마 토글 버튼 추가.
- `static/app.js` — `applyTheme`/`toggleTheme` 추가, 버튼 배선, 파일 상단에서 저장된 테마 복원.

---

### Task 1: 토큰 기반 CSS + 컴포넌트 단장 (라이트/다크 토큰 정의 포함)

**Files:**
- Modify: `static/index.html`

**Interfaces:**
- Consumes: 기존 요소 셀렉터(`header`, `#grid`, `.cell`, `.cell.today`, `.daynum`, `.badge`, `.files`, `.file`, `#msg`, `#panel`, `#weekhead`, `button`, `input`).
- Produces: `:root` 라이트 토큰과 `:root[data-theme="dark"]` 다크 토큰. (Task 2의 JS가 `data-theme` 속성을 토글한다.)

- [ ] **Step 1: index.html의 `<style>` 블록 전체 교체**

현재 `<style> ... </style>` 안의 내용을 아래로 통째로 교체한다(셀렉터 이름은 유지, 토큰화).

```css
  :root {
    --accent: #0066cc;
    --accent-hover: #1a78d6;
    --bg: #ffffff;
    --surface: #f5f5f7;
    --surface-2: #ffffff;
    --border: #d2d2d7;
    --text: #1d1d1f;
    --text-dim: #6e6e73;
    --on-accent: #ffffff;
    --space-1: 4px;
    --space-2: 8px;
    --space-3: 16px;
    --space-4: 24px;
    --radius-sm: 8px;
    --radius-pill: 9999px;
    --shadow: 0 4px 16px rgba(0,0,0,0.08);
  }
  :root[data-theme="dark"] {
    --accent: #2997ff;
    --accent-hover: #4aa8ff;
    --bg: #1d1d1f;
    --surface: #2c2c2e;
    --surface-2: #242426;
    --border: #3a3a3c;
    --text: #f5f5f7;
    --text-dim: #a1a1a6;
    --on-accent: #ffffff;
  }

  body {
    font-family: -apple-system, BlinkMacSystemFont, "Malgun Gothic", sans-serif;
    margin: var(--space-4);
    background: var(--bg);
    color: var(--text);
  }
  header {
    display: flex; align-items: center; gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  #label { font-size: 18px; letter-spacing: -0.02em; }

  button {
    font-family: inherit;
    background: var(--accent);
    color: var(--on-accent);
    border: none;
    border-radius: var(--radius-pill);
    padding: 6px 14px;
    cursor: pointer;
    transition: background 0.15s, transform 0.05s;
  }
  button:hover { background: var(--accent-hover); }
  button:active { transform: scale(0.98); }

  input {
    font-family: inherit;
    background: var(--surface-2);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 10px;
    outline: none;
    transition: box-shadow 0.15s, border-color 0.15s;
  }
  input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 30%, transparent);
  }

  #weekhead {
    display: grid; grid-template-columns: repeat(7, 1fr); gap: var(--space-1);
    font-weight: 600; color: var(--text-dim); margin-bottom: var(--space-1);
  }
  #grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: var(--space-1); }
  .cell {
    border: 1px solid var(--border);
    background: var(--surface-2);
    border-radius: var(--radius-sm);
    min-height: 76px; padding: var(--space-1); font-size: 12px;
  }
  .cell.today { outline: 2px solid var(--accent); outline-offset: -1px; }
  .daynum { color: var(--text-dim); margin-bottom: 2px; }

  .badge {
    display: inline-block;
    background: var(--accent); color: var(--on-accent);
    border-radius: var(--radius-pill);
    padding: 1px 8px; font-size: 11px; cursor: pointer;
  }
  .files { margin-top: var(--space-1); }
  .file {
    cursor: pointer; color: var(--accent); display: block;
    padding: 1px 0;
  }
  .file:hover { text-decoration: underline; }

  #msg { color: #ff453a; }

  #panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    padding: var(--space-3); margin: var(--space-2) 0; max-width: 600px;
  }
  #panel hr { border: none; border-top: 1px solid var(--border); margin: var(--space-3) 0; }
  #pathlist { list-style: none; padding: 0; }
```

NOTE: `#panel`은 기존에 인라인 style 속성(`style="display:none;..."`)을 갖고 있다. display 토글은 JS가 인라인으로 제어하므로 그대로 두고, 위 CSS의 `#panel` 규칙이 색·여백을 입힌다. 단, 기존 인라인의 `border/padding/margin/max-width`와 중복되어도 무방하다(인라인이 우선). 깔끔하게 하려면 다음 Step에서 인라인을 정리한다.

- [ ] **Step 2: #panel 인라인 스타일 정리(선택적 일관성)**

`index.html`의 `<div id="panel" style="display:none;border:1px solid #ccc;padding:12px;margin:8px 0;max-width:600px;">`
를 다음으로 바꾼다(시각 속성은 CSS로 옮기고 display만 인라인 유지):
```html
  <div id="panel" style="display:none;">
```
그리고 `#panel` 내부의 인라인 스타일 잔재도 정리:
- `<ul id="pathlist" style="list-style:none;padding:0;">` → `<ul id="pathlist">` (CSS의 #pathlist 규칙이 처리)
- `<span id="panelMsg" style="color:#b91c1c;">` → `<span id="panelMsg" style="color:#ff453a;">` (유지 가능, 색만 통일)
- `#weekhead`의 인라인 `style="display:grid;..."` → `<div id="weekhead">` (CSS의 #weekhead 규칙이 처리)
- `<input id="extInput" ... size="12">`의 size 속성은 유지.

- [ ] **Step 3: 검증**

브라우저 없이 정합성만 확인:
Run: `python -c "print('check')"` 대신 파일 열어 육안 확인은 생략하고, 다음으로 회귀만 확인:
Run: `python -m pytest tests/ -q`
Expected: 18 passed (서버/로직 무관).
(외관 자체는 Task 2 완료 후 브라우저에서 한 번에 수동 확인.)

- [ ] **Step 4: Commit**

```bash
git add static/index.html
git commit -m "feat: token-based Apple-style CSS for calendar UI"
```

---

### Task 2: 다크모드 토글 + 기억

**Files:**
- Modify: `static/index.html`
- Modify: `static/app.js`

**Interfaces:**
- Consumes: Task 1의 `:root[data-theme="dark"]` 토큰. 기존 app.js 이벤트 배선부.
- Produces: `applyTheme(theme)` / `toggleTheme()` 함수, 헤더 `#themeToggle` 버튼.

- [ ] **Step 1: index.html 헤더에 토글 버튼 추가**

헤더의 `<input id="search" type="text" placeholder="파일명 검색">` 줄 다음에 추가:
```html
    <button id="themeToggle" title="라이트/다크 전환">☾</button>
```

- [ ] **Step 2: app.js에 테마 로직 추가**

`app.js` 최상단(기존 `let data = {};` 보다 위)에 추가한다. 깜빡임을 줄이려 즉시 복원도 수행:

```javascript
function applyTheme(theme) {
  document.documentElement.setAttribute("data-theme", theme);
  try { localStorage.setItem("theme", theme); } catch (_) {}
  const btn = document.getElementById("themeToggle");
  if (btn) btn.textContent = theme === "dark" ? "☀" : "☾";
}

function toggleTheme() {
  const cur = document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light";
  applyTheme(cur === "dark" ? "light" : "dark");
}

(function restoreTheme() {
  let saved = "light";
  try { saved = localStorage.getItem("theme") || "light"; } catch (_) {}
  document.documentElement.setAttribute("data-theme", saved);
})();
```

NOTE: 위 IIFE는 `data-theme`만 즉시 세팅한다(버튼 아이콘은 DOM 로드 후 배선 단계에서 맞춘다).

- [ ] **Step 3: app.js 이벤트 배선에 토글 연결 + 버튼 아이콘 초기화**

`app.js`의 이벤트 등록부(예: `document.getElementById("search").oninput = ...` 부근)에 추가:
```javascript
document.getElementById("themeToggle").onclick = toggleTheme;
applyTheme(document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light");
```
(마지막 줄은 복원된 테마에 맞춰 버튼 아이콘을 한 번 맞춘다. `data-theme`은 이미 IIFE가 세팅했으므로 값은 보존된다.)

- [ ] **Step 4: 검증**

Run: `node --check static/app.js`
Expected: 오류 없음.
Run: `python -m pytest tests/ -q`
Expected: 18 passed.

- [ ] **Step 5: Commit**

```bash
git add static/index.html static/app.js
git commit -m "feat: dark mode toggle with localStorage persistence"
```

---

## Self-Review

**Spec coverage:**
- 디자인 토큰(:root 색/간격/모서리/폰트) → Task 1 Step 1. ✓
- 컴포넌트 단장(버튼 알약·press, 칸 카드·today 링, 배지, 파일 hover, 입력 focus 링, 패널 카드) → Task 1 Step 1. ✓
- 라이트/다크 토큰 → Task 1 Step 1 (`:root` / `:root[data-theme="dark"]`). ✓
- 토글 버튼 + 전환 → Task 2 Step 1, 3 (`toggleTheme`). ✓
- localStorage 저장·복원·없으면 light → Task 2 Step 2 (`applyTheme` 저장, IIFE 복원). ✓
- localStorage try/catch → Task 2 Step 2. ✓
- 구조·기능 불변 → CSS 셀렉터 유지, JS는 추가만. ✓
- 시스템 폰트, 웹폰트 없음 → Task 1 Step 1 body font-family. ✓
- 검증(node --check, pytest 18) → Task 1 Step 3, Task 2 Step 4. ✓

**Placeholder scan:** 없음. 모든 코드 스텝에 실제 코드.

**Type consistency:** `applyTheme`/`toggleTheme` 정의(Task 2 Step 2)와 사용(Step 3) 일치. 버튼 id `themeToggle`이 index.html(Task 2 Step 1)·app.js(Step 2,3) 일치. CSS 토큰명(`--accent`,`--bg`,`--surface`,`--border`,`--text`,`--text-dim`,`--radius-sm`,`--radius-pill`,`--space-*`)이 정의·사용 일치. `data-theme` 값 "dark"/"light"가 CSS 셀렉터·JS 전반 일치.

**Note (color-mix):** input:focus의 `color-mix(in srgb, ...)`는 최신 브라우저(크롬/엣지 111+) 지원. 개인용 최신 브라우저 전제라 사용. 미지원 시 단순 box-shadow로 폴백 가능하나 YAGNI로 두지 않음.
