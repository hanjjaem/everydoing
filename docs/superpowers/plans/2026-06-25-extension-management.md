# 파일 유형(확장자) UI 선택 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 설정 패널에서 스캔 대상 확장자를 체크박스로 켜고 끄고 임의 확장자를 직접 추가해 config.json에 저장한다.

**Architecture:** `everything_client.py`에 확장자 정규화 순수 함수를 더하고, `server.py`의 `/api/config`가 extensions를 함께 조회/저장하도록 확장한다. UI는 기존 설정 패널에 확장자 체크박스 영역(프리셋 ∪ 현재) + 직접 추가 입력을 더하고, 저장 시 폴더와 확장자를 함께 보낸다.

**Tech Stack:** Python 3 표준 라이브러리만, 바닐라 HTML/JS.

## Global Constraints

- 외부 파이썬 패키지 금지 — 표준 라이브러리만.
- Windows 전용.
- `config.json`이 설정 단일 출처. `POST /api/config`는 받은 키만 교체하고 나머지 키(everything_host, everything_port, server_port)는 보존한다.
- 기존 함수·라우트·캘린더 UI는 그대로 동작해야 한다(`/api/files`, `/api/open`, scan_paths 처리, 폴더 패널).
- 공개 함수 시그니처에는 타입 힌트를 단다(기존 컨벤션).
- 프리셋 확장자: `["hwp", "hwpx", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf"]` (정확히 이 순서/값).

---

## File Structure

- `everything_client.py` — `normalize_extension`, `normalize_extensions` 추가.
- `server.py` — `GET /api/config` 응답에 extensions 추가, `_handle_save_config`가 extensions도 처리.
- `static/index.html` — 설정 패널에 "파일 유형" 영역(체크박스 컨테이너 + 직접추가 입력) 추가.
- `static/app.js` — 확장자 체크박스 렌더(프리셋 ∪ 현재), 직접 추가, 저장 시 extensions 포함.
- `tests/test_everything_client.py` — 정규화 함수 단위 테스트.

---

### Task 1: 확장자 정규화 순수 함수

**Files:**
- Modify: `everything_client.py`
- Test: `tests/test_everything_client.py`

**Interfaces:**
- Consumes: (없음)
- Produces:
  - `normalize_extension(raw: str) -> "str | None"` — 앞뒤 공백 제거, 맨 앞 점 한 개 제거, 소문자화. 결과가 빈 문자열이면 None. (입력이 None이면 None)
  - `normalize_extensions(raws: "list") -> "list[str]"` — 각 항목을 normalize_extension으로 변환, None 제거, 순서 유지하며 중복 제거.

- [ ] **Step 1: Write the failing tests**

```python
# tests/test_everything_client.py 에 추가
from everything_client import normalize_extension, normalize_extensions


def test_normalize_extension_strips_dot_and_lowercases():
    assert normalize_extension(" .HWP ") == "hwp"


def test_normalize_extension_plain():
    assert normalize_extension("docx") == "docx"


def test_normalize_extension_empty_returns_none():
    assert normalize_extension("") is None
    assert normalize_extension("   ") is None
    assert normalize_extension(".") is None


def test_normalize_extensions_dedupes_keeps_order_drops_none():
    assert normalize_extensions(["PDF", ".pdf", "hwp", "", "docx"]) == ["pdf", "hwp", "docx"]
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_everything_client.py -v`
Expected: the 4 new tests FAIL (cannot import name 'normalize_extension'); existing 14 still pass.

- [ ] **Step 3: Write the implementation**

```python
# everything_client.py 에 추가

def normalize_extension(raw: str) -> "str | None":
    if raw is None:
        return None
    s = raw.strip()
    if s.startswith("."):
        s = s[1:]
    s = s.strip().lower()
    return s or None


def normalize_extensions(raws: "list") -> "list[str]":
    out = []
    for r in raws:
        e = normalize_extension(r)
        if e and e not in out:
            out.append(e)
    return out
```

- [ ] **Step 4: Run all tests**

Run: `python -m pytest tests/ -v`
Expected: 18 passed.

- [ ] **Step 5: Commit**

```bash
git add everything_client.py tests/test_everything_client.py
git commit -m "feat: extension normalization helpers"
```

---

### Task 2: 서버 — /api/config 가 extensions 조회/저장

**Files:**
- Modify: `server.py`

**Interfaces:**
- Consumes: `ec.normalize_extensions`, 기존 `ec.load_config`, `ec.save_config`, 전역 `CONFIG`.
- Produces:
  - `GET /api/config` → `{"scan_paths": [...], "extensions": [...]}`
  - `POST /api/config` → 기존 scan_paths 처리 유지. 추가로 body에 `extensions`가 있으면 list 검증(아니면 400) 후 `normalize_extensions`로 정규화해 config의 `extensions` 키 교체. body에 extensions 없으면 기존 값 보존.

통합이라 자동 테스트 없음 — 수동 검증 + 기존 단위 테스트 회귀.

- [ ] **Step 1: GET /api/config 응답에 extensions 추가**

`server.py`의 `do_GET` 안 기존 분기:
```python
        if path == "/api/config":
            self._send_json(200, {"scan_paths": CONFIG.get("scan_paths", [])})
            return
```
를 다음으로 바꾼다:
```python
        if path == "/api/config":
            self._send_json(200, {
                "scan_paths": CONFIG.get("scan_paths", []),
                "extensions": CONFIG.get("extensions", []),
            })
            return
```

- [ ] **Step 2: _handle_save_config 가 extensions 처리**

현재 메서드:
```python
    def _handle_save_config(self, data):
        global CONFIG
        scan_paths = data.get("scan_paths")
        if not isinstance(scan_paths, list):
            self._send_json(400, {"error": "scan_paths(list) 필요"})
            return
        try:
            cfg = ec.load_config()
            cfg["scan_paths"] = scan_paths
            ec.save_config(cfg)
            CONFIG = cfg
        except OSError as e:
            self._send_json(500, {"error": str(e)})
            return
        self._send_json(200, {"ok": True})
```
를 다음으로 바꾼다(extensions가 있으면 검증·정규화·교체, 없으면 그대로):
```python
    def _handle_save_config(self, data):
        global CONFIG
        scan_paths = data.get("scan_paths")
        if not isinstance(scan_paths, list):
            self._send_json(400, {"error": "scan_paths(list) 필요"})
            return
        extensions = data.get("extensions")
        if extensions is not None and not isinstance(extensions, list):
            self._send_json(400, {"error": "extensions(list) 필요"})
            return
        try:
            cfg = ec.load_config()
            cfg["scan_paths"] = scan_paths
            if extensions is not None:
                cfg["extensions"] = ec.normalize_extensions(extensions)
            ec.save_config(cfg)
            CONFIG = cfg
        except OSError as e:
            self._send_json(500, {"error": str(e)})
            return
        self._send_json(200, {"ok": True})
```

- [ ] **Step 3: 수동 검증 (사용자 확인)**

전제: Everything HTTP 서버 켜짐. `python server.py` 실행.
- `GET /api/config` → scan_paths와 extensions 둘 다 온다.
- `POST /api/config` with `{"scan_paths":[...], "extensions":["PDF",".pdf","hwp"]}` →
  `{"ok":true}`, config.json의 extensions가 `["pdf","hwp"]`로 정규화 저장된다.
- `POST /api/config` with `{"scan_paths":[...]}` (extensions 없음) → 기존 extensions 보존.
- 기존 단위 테스트 회귀: `python -m pytest tests/ -q` → 18 passed.

- [ ] **Step 4: Commit**

```bash
git add server.py
git commit -m "feat: config API reads and saves extensions"
```

---

### Task 3: UI — 확장자 체크박스 + 직접 추가

**Files:**
- Modify: `static/index.html`
- Modify: `static/app.js`

**Interfaces:**
- Consumes: `GET /api/config`(scan_paths+extensions), `POST /api/config`(scan_paths+extensions), 기존 `openPanel`/`savePaths`/`editPaths`/`load`.
- Produces: 설정 패널의 확장자 체크박스 영역. 저장 시 체크된 확장자가 config.extensions로 저장됨.

UI라 수동 검증.

- [ ] **Step 1: index.html — 패널에 파일 유형 영역 추가**

설정 패널(`<div id="panel" ...>`) 안, `<button id="addPath">폴더 추가</button>` 줄과
`<button id="savePaths">저장</button>` 줄 **사이**에 다음을 추가:

```html
    <hr>
    <strong>파일 유형</strong>
    <div id="extlist"></div>
    <input id="extInput" type="text" placeholder="확장자 (예: csv)" size="12">
    <button id="addExt">추가</button>
```

- [ ] **Step 2: app.js — 확장자 상태/렌더/추가 로직**

`app.js`에서 기존 `let editPaths = [];` 줄 바로 다음에 추가:

```javascript
let editExts = [];
const PRESET_EXTS = ["hwp", "hwpx", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf"];

function renderExts() {
  const box = document.getElementById("extlist");
  box.innerHTML = "";
  const all = PRESET_EXTS.slice();
  editExts.forEach(e => { if (!all.includes(e)) all.push(e); });
  all.forEach(ext => {
    const label = document.createElement("label");
    label.style.marginRight = "10px";
    const cb = document.createElement("input");
    cb.type = "checkbox";
    cb.checked = editExts.includes(ext);
    cb.onchange = () => {
      if (cb.checked) { if (!editExts.includes(ext)) editExts.push(ext); }
      else { editExts = editExts.filter(x => x !== ext); }
    };
    label.appendChild(cb);
    label.appendChild(document.createTextNode(" " + ext));
    box.appendChild(label);
  });
}

function addExt() {
  const input = document.getElementById("extInput");
  let v = input.value.trim().replace(/^\./, "").toLowerCase();
  if (v && !editExts.includes(v)) { editExts.push(v); renderExts(); }
  input.value = "";
}
```

- [ ] **Step 3: app.js — openPanel 에서 extensions 로드, savePaths 에서 전송**

기존 `openPanel`의 본문에서 `editPaths = (cfg.scan_paths || []).slice();` 다음 줄에 추가:
```javascript
  editExts = (cfg.extensions || []).slice();
  renderExts();
```
그리고 기존 `savePaths`의 fetch body를 다음으로 바꾼다:
```javascript
    body: JSON.stringify({ scan_paths: editPaths, extensions: editExts }),
```
끝으로, 기존 이벤트 핸들러 등록부(예: `document.getElementById("addPath").onclick = addPath;` 부근)에 추가:
```javascript
document.getElementById("addExt").onclick = addExt;
```

- [ ] **Step 4: 수동 검증 (사용자 확인)**

`python server.py` 실행 후 브라우저 `http://localhost:5000`:
1. "폴더 설정" → 패널에 "파일 유형" 체크박스가 보이고, 현재 config의 확장자가 체크돼 있다.
2. 체크 해제 → 저장 → 캘린더에서 그 유형이 빠진다.
3. 직접 추가 입력(예: `csv`) → "추가" → 체크박스 목록에 csv가 체크된 채 생긴다.
4. 저장 → config.json의 extensions가 갱신되고 캘린더가 그에 맞게 갱신된다.
5. `.PDF` 처럼 점·대문자로 넣어도 `pdf`로 정규화되어 중복 안 생긴다.

- [ ] **Step 5: Commit**

```bash
git add static/index.html static/app.js
git commit -m "feat: file-type checkboxes in settings panel"
```

---

## Self-Review

**Spec coverage:**
- 프리셋 체크박스 + 현재 config 확장자 체크 → Task 3 (renderExts: PRESET ∪ editExts). ✓
- 프리셋에 없는 현재 확장자도 표시·체크 → Task 3 (all에 editExts 추가). ✓
- 직접 추가(정규화·중복무시) → Task 3 (addExt) + 클라이언트 정규화 일치. ✓
- 저장은 폴더와 공용 버튼, 함께 저장 → Task 3 (savePaths body에 scan_paths+extensions). ✓
- normalize_extension/normalize_extensions 순수 함수 + 단위 테스트 → Task 1. ✓
- GET /api/config extensions 포함 → Task 2 Step 1. ✓
- POST /api/config extensions 정규화 교체, 없으면 보존, 다른 키 보존 → Task 2 Step 2. ✓
- extensions 비list → 400 → Task 2 Step 2. ✓
- 정규화 후 빈 리스트 그대로 저장 → Task 2 (normalize_extensions가 [] 반환, 그대로 저장). ✓
- 기존 동작 보존 → Task 2가 scan_paths 처리·400·500 경로 유지. ✓

**Placeholder scan:** 없음. 모든 코드 스텝에 실제 코드.

**Type consistency:** `normalize_extension`/`normalize_extensions` 이름이 Task 1 정의·Task 2 사용처 일치. `/api/config` 응답·요청 키 `scan_paths`,`extensions`가 server(Task 2)·app.js(Task 3) 일치. 클라이언트 정규화(`replace(/^\./,"").toLowerCase()`)가 서버 `normalize_extension`과 동일 규칙 — 서버가 최종 정규화하므로 클라이언트는 표시 일관성용. `editExts`/`renderExts`/`addExt`/`PRESET_EXTS` 이름 Task 3 내부 일관, 기존 `editPaths`와 충돌 없음.

**Note:** Task 1 테스트의 import는 기존 import 줄에 이어 붙인다(중복 없게).
