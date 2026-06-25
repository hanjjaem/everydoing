# 스캔 폴더 UI 관리 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 캘린더 웹 화면에서 스캔 대상 폴더를 윈도우 탐색기 선택창으로 추가/삭제하고 config.json에 저장한다.

**Architecture:** `everything_client.py`에 config 저장·경로 추가/삭제 순수 함수를 더하고, `server.py`에 설정 조회/저장/폴더선택 API 3개를 더한다. 폴더 선택은 표준 tkinter 다이얼로그를 별도 헬퍼 스크립트(`pick_folder.py`)로 분리해 subprocess로 띄운다. UI는 헤더의 "폴더 설정" 버튼이 여는 패널에서 목록 편집 후 저장하면 캘린더를 다시 불러온다.

**Tech Stack:** Python 3 표준 라이브러리만 (`http.server`, `json`, `subprocess`, `tkinter`), 바닐라 HTML/JS.

## Global Constraints

- 외부 파이썬 패키지 금지 — 표준 라이브러리만 사용한다 (tkinter 포함, 표준).
- 플랫폼은 Windows 전용.
- 모든 설정은 `config.json` 한 곳에서 읽고 쓴다.
- `POST /api/config`는 `scan_paths` 키만 교체하고 나머지 키(everything_port, extensions, server_port 등)는 보존한다.
- 기존 함수(`build_search_query`, `filetime_to_date`, `group_by_date`, `load_config`, `fetch_files`)와 기존 라우트(`/api/files`, `/api/open`, 정적 서빙)는 그대로 동작해야 한다.
- 공개 함수 시그니처에는 타입 힌트를 단다(기존 코드 컨벤션).

---

## File Structure

- `everything_client.py` — 기존에 더해 `save_config`, `add_scan_path`, `remove_scan_path` 추가.
- `pick_folder.py` — 신규. tkinter 폴더 선택창을 띄워 고른 경로를 stdout으로 출력하는 독립 스크립트.
- `server.py` — `GET/POST /api/config`, `POST /api/pick-folder` 라우트 추가.
- `static/index.html` — "폴더 설정" 버튼 + 설정 패널 마크업/스타일 추가.
- `static/app.js` — 패널 열기/닫기, 목록 렌더, 폴더 추가/삭제, 저장 로직 추가.
- `tests/test_everything_client.py` — `add_scan_path`, `remove_scan_path`, `save_config`+`load_config` 왕복 테스트 추가.

---

### Task 1: config 저장 및 경로 추가/삭제 순수 함수

**Files:**
- Modify: `everything_client.py`
- Test: `tests/test_everything_client.py`

**Interfaces:**
- Consumes: 기존 `load_config(path="config.json") -> dict`
- Produces:
  - `add_scan_path(scan_paths: "list[str]", new_path: str) -> "list[str]"` — new_path가 빈 문자열이거나 이미 목록에 있으면 원본 그대로(중복 없이), 아니면 끝에 추가한 새 리스트 반환.
  - `remove_scan_path(scan_paths: "list[str]", target: str) -> "list[str]"` — target과 같은 항목을 제거한 새 리스트 반환. 없으면 그대로.
  - `save_config(config: dict, path: str = "config.json") -> None` — config를 UTF-8 JSON으로 들여쓰기 2칸 저장.

- [ ] **Step 1: Write the failing tests**

```python
# tests/test_everything_client.py 에 추가
import json
import os
import tempfile
from everything_client import add_scan_path, remove_scan_path, save_config, load_config


def test_add_scan_path_appends_new():
    assert add_scan_path(["C:\\A"], "C:\\B") == ["C:\\A", "C:\\B"]


def test_add_scan_path_ignores_duplicate():
    assert add_scan_path(["C:\\A"], "C:\\A") == ["C:\\A"]


def test_add_scan_path_ignores_empty():
    assert add_scan_path(["C:\\A"], "") == ["C:\\A"]


def test_remove_scan_path_removes():
    assert remove_scan_path(["C:\\A", "C:\\B"], "C:\\A") == ["C:\\B"]


def test_remove_scan_path_absent_noop():
    assert remove_scan_path(["C:\\A"], "C:\\X") == ["C:\\A"]


def test_save_then_load_config_roundtrip():
    cfg = {"scan_paths": ["C:\\A"], "server_port": 5000}
    fd, path = tempfile.mkstemp(suffix=".json")
    os.close(fd)
    try:
        save_config(cfg, path)
        assert load_config(path) == cfg
    finally:
        os.remove(path)
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `python -m pytest tests/test_everything_client.py -v`
Expected: the 6 new tests FAIL (cannot import name 'add_scan_path' ...). Existing 8 still pass.

- [ ] **Step 3: Write the implementation**

```python
# everything_client.py 에 추가 (json 은 이미 import 되어 있음 — 중복 import 추가하지 말 것)

def add_scan_path(scan_paths: "list[str]", new_path: str) -> "list[str]":
    if not new_path or new_path in scan_paths:
        return list(scan_paths)
    return list(scan_paths) + [new_path]


def remove_scan_path(scan_paths: "list[str]", target: str) -> "list[str]":
    return [p for p in scan_paths if p != target]


def save_config(config: dict, path: str = "config.json") -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(config, f, ensure_ascii=False, indent=2)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `python -m pytest tests/ -v`
Expected: 14 passed.

- [ ] **Step 5: Commit**

```bash
git add everything_client.py tests/test_everything_client.py
git commit -m "feat: config save and scan-path add/remove helpers"
```

---

### Task 2: 폴더 선택 다이얼로그 헬퍼 스크립트

**Files:**
- Create: `pick_folder.py`

**Interfaces:**
- Consumes: (없음 — 독립 실행 스크립트)
- Produces: `python pick_folder.py` 실행 시 윈도우 폴더 선택창을 띄우고, 선택한 폴더의 절대경로를 stdout에 한 줄 출력한다. 취소하면 빈 출력(빈 줄). 이 스크립트는 Task 3의 `/api/pick-folder`가 subprocess로 호출한다.

이 태스크는 GUI라 자동 테스트 없음 — 수동 검증.

- [ ] **Step 1: Create `pick_folder.py`**

```python
# pick_folder.py
# 폴더 선택창을 띄워 고른 경로를 stdout으로 출력한다. 취소 시 빈 줄.
import tkinter
from tkinter import filedialog


def main():
    root = tkinter.Tk()
    root.withdraw()
    root.attributes("-topmost", True)
    path = filedialog.askdirectory()
    root.destroy()
    print(path or "")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: 수동 검증 (사용자 확인)**

Run: `python pick_folder.py`
Expected: 폴더 선택창이 뜬다. 폴더를 고르면 그 경로가 출력되고, 취소하면 빈 줄이 출력된다.

- [ ] **Step 3: Commit**

```bash
git add pick_folder.py
git commit -m "feat: tkinter folder picker helper script"
```

---

### Task 3: 서버 API — config 조회/저장, 폴더 선택

**Files:**
- Modify: `server.py`

**Interfaces:**
- Consumes: `load_config`, `save_config`, 기존 `CONFIG` 전역. `pick_folder.py` (subprocess).
- Produces:
  - `GET /api/config` → `{"scan_paths": [...]}`
  - `POST /api/config` body `{"scan_paths": [...]}` → 디스크의 config를 다시 읽어 `scan_paths`만 교체 후 저장하고 전역 CONFIG도 갱신. 응답 `{"ok": true}`. 본문에 `scan_paths`(list) 없으면 400.
  - `POST /api/pick-folder` → `pick_folder.py`를 subprocess로 실행해 선택 경로 반환 `{"path": "..."}`. 실패 시 500 + `{"error": ...}`.

이 태스크는 통합(HTTP+subprocess)이라 자동 단위 테스트 없음 — 수동 검증.

- [ ] **Step 1: do_GET에 /api/config 분기 추가**

`server.py`의 `do_GET` 안, `if path == "/api/files":` 블록 바로 다음(아직 `_serve_static` 호출 전)에 추가:

```python
        if path == "/api/config":
            self._send_json(200, {"scan_paths": CONFIG.get("scan_paths", [])})
            return
```

- [ ] **Step 2: do_POST에 /api/config 와 /api/pick-folder 분기 추가**

현재 `do_POST`는 `/api/open`만 처리한다. 맨 위의 경로 분기를 바꾼다. 기존:

```python
    def do_POST(self):
        if urlparse(self.path).path != "/api/open":
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", 0))
        try:
            data = json.loads(self.rfile.read(length).decode("utf-8"))
        except (json.JSONDecodeError, UnicodeDecodeError):
            self._send_json(400, {"error": "invalid body"})
            return
        target = data.get("path", "")
        ...
```

를 다음으로 바꾼다 (본문 파싱을 공통으로 끌어올리고 라우팅):

```python
    def do_POST(self):
        route = urlparse(self.path).path
        if route not in ("/api/open", "/api/config", "/api/pick-folder"):
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length) if length else b""
        try:
            data = json.loads(raw.decode("utf-8")) if raw else {}
        except (json.JSONDecodeError, UnicodeDecodeError):
            self._send_json(400, {"error": "invalid body"})
            return
        if route == "/api/open":
            self._handle_open(data)
        elif route == "/api/config":
            self._handle_save_config(data)
        else:
            self._handle_pick_folder()
```

그리고 기존 open 처리 본문을 `_handle_open` 메서드로 옮긴다(로직 동일):

```python
    def _handle_open(self, data):
        target = data.get("path", "")
        if not os.path.exists(target):
            self._send_json(404, {"error": "파일을 찾을 수 없습니다"})
            return
        if data.get("mode") == "folder":
            subprocess.Popen(["explorer", "/select,", target])
        else:
            os.startfile(target)
        self._send_json(200, {"ok": True})
```

- [ ] **Step 3: config 저장과 폴더선택 핸들러 추가**

`Handler` 클래스 안에 메서드 추가. `import everything_client as ec`는 파일 상단에 이미 있음. `global CONFIG` 갱신을 위해 모듈 전역 CONFIG를 재대입한다:

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

    def _handle_pick_folder(self):
        helper = os.path.join(os.path.dirname(__file__), "pick_folder.py")
        try:
            out = subprocess.run(
                [sys.executable, helper],
                capture_output=True, text=True, timeout=120,
            )
        except (OSError, subprocess.SubprocessError) as e:
            self._send_json(500, {"error": str(e)})
            return
        self._send_json(200, {"path": out.stdout.strip()})
```

- [ ] **Step 4: 상단 import에 sys 추가**

`server.py` 상단 import 블록에 `import sys`가 없으면 추가한다 (`import os` 옆).

- [ ] **Step 5: 수동 검증 (사용자 확인)**

전제: Everything HTTP 서버 켜짐. `python server.py` 실행.
- `curl http://localhost:5000/api/config` → 현재 scan_paths JSON.
- (브라우저 콘솔 또는 curl) `POST /api/pick-folder` → 폴더 선택창이 뜨고 고른 경로가 `{"path":...}`로 온다.
- `POST /api/config` with `{"scan_paths":[...]}` → `{"ok":true}`, config.json이 갱신된다.
또한 기존 unit 테스트 회귀: `python -m pytest tests/ -q` → 14 passed.

- [ ] **Step 6: Commit**

```bash
git add server.py
git commit -m "feat: config get/save and folder-picker API routes"
```

---

### Task 4: UI — 폴더 설정 패널

**Files:**
- Modify: `static/index.html`
- Modify: `static/app.js`

**Interfaces:**
- Consumes: `GET /api/config`, `POST /api/config`, `POST /api/pick-folder`, 기존 `load()`.
- Produces: 헤더의 "폴더 설정" 버튼이 여는 패널. 목록 편집 후 저장하면 config 저장 + `load()` 재호출로 캘린더 갱신.

UI라 수동 검증.

- [ ] **Step 1: index.html에 버튼과 패널 추가**

헤더의 `<button id="refresh">새로고침</button>` 바로 다음 줄에 추가:

```html
    <button id="settings">폴더 설정</button>
```

그리고 `<div id="msg"></div>` 바로 다음에 패널 마크업 추가:

```html
  <div id="panel" style="display:none;border:1px solid #ccc;padding:12px;margin:8px 0;max-width:600px;">
    <strong>스캔 폴더</strong>
    <ul id="pathlist" style="list-style:none;padding:0;"></ul>
    <button id="addPath">폴더 추가</button>
    <button id="savePaths">저장</button>
    <button id="closePanel">닫기</button>
    <span id="panelMsg" style="color:#b91c1c;"></span>
  </div>
```

- [ ] **Step 2: app.js에 패널 로직 추가**

`app.js`의 맨 끝(기존 `load();` 호출 바로 위)에 추가:

```javascript
let editPaths = [];

function renderPaths() {
  const ul = document.getElementById("pathlist");
  ul.innerHTML = "";
  editPaths.forEach((p, i) => {
    const li = document.createElement("li");
    li.textContent = p + " ";
    const x = document.createElement("button");
    x.textContent = "×";
    x.onclick = () => { editPaths.splice(i, 1); renderPaths(); };
    li.appendChild(x);
    ul.appendChild(li);
  });
}

async function openPanel() {
  document.getElementById("panelMsg").textContent = "";
  const res = await fetch("/api/config");
  const cfg = await res.json();
  editPaths = (cfg.scan_paths || []).slice();
  renderPaths();
  document.getElementById("panel").style.display = "block";
}

async function addPath() {
  const res = await fetch("/api/pick-folder", { method: "POST" });
  if (!res.ok) { document.getElementById("panelMsg").textContent = "폴더 선택 실패"; return; }
  const { path } = await res.json();
  if (path && !editPaths.includes(path)) { editPaths.push(path); renderPaths(); }
}

async function savePaths() {
  const res = await fetch("/api/config", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ scan_paths: editPaths }),
  });
  if (!res.ok) { document.getElementById("panelMsg").textContent = "저장 실패"; return; }
  document.getElementById("panel").style.display = "none";
  load();
}

document.getElementById("settings").onclick = openPanel;
document.getElementById("addPath").onclick = addPath;
document.getElementById("savePaths").onclick = savePaths;
document.getElementById("closePanel").onclick = () => { document.getElementById("panel").style.display = "none"; };
```

- [ ] **Step 3: 수동 검증 (사용자 확인)**

`python server.py` 실행 후 브라우저 `http://localhost:5000`:
1. "폴더 설정" 클릭 → 패널에 현재 폴더 목록이 뜬다.
2. "폴더 추가" → 폴더 선택창 → 고른 폴더가 목록에 추가된다.
3. 항목 × → 목록에서 사라진다.
4. "저장" → 패널 닫히고 캘린더가 갱신된다(새 폴더의 파일이 잡힌다). config.json도 바뀌어 있다.
5. "닫기" → 저장 없이 패널만 닫힌다.

- [ ] **Step 4: Commit**

```bash
git add static/index.html static/app.js
git commit -m "feat: scan-folder settings panel in calendar UI"
```

---

## Self-Review

**Spec coverage:**
- "폴더 설정" 버튼 + 패널 → Task 4. ✓
- 현재 폴더 목록 + 삭제(×) → Task 4 (renderPaths). ✓
- 폴더 추가 = 탐색기 선택창 → Task 2(pick_folder.py) + Task 3(/api/pick-folder) + Task 4(addPath). ✓
- 저장 → config.json 갱신 + 캘린더 갱신 → Task 3(/api/config POST) + Task 4(savePaths→load). ✓
- GET/POST /api/config, POST /api/pick-folder → Task 3. ✓
- scan_paths 키만 교체, 나머지 보존 → Task 3 (_handle_save_config: load 후 키 교체). ✓
- add/remove/중복제거 순수 함수 + 단위 테스트 → Task 1. ✓
- 다이얼로그 취소 → 빈 경로 → Task 2(print "" ) + Task 4(addPath: path 비면 무시). ✓
- 저장 실패 메시지 → Task 3(500) + Task 4(panelMsg). ✓
- 다이얼로그 실패 메시지 → Task 3(500) + Task 4. ✓
- 기존 함수/라우트 보존 → Task 3가 _handle_open으로 로직 동일 이동. ✓

**Placeholder scan:** 없음. 모든 코드 스텝에 실제 코드 포함.

**Type consistency:** `add_scan_path`/`remove_scan_path`/`save_config` 이름이 Task 1 정의와 Task 3 사용처에서 일치. `/api/config` 응답 키 `scan_paths`, `/api/pick-folder` 응답 키 `path`가 server(Task 3)와 app.js(Task 4)에서 일치. `_handle_open`/`_handle_save_config`/`_handle_pick_folder` 메서드명 Task 3 내부 일관.

**Note:** Task 1의 test 추가 import(`json, os, tempfile`)는 테스트 파일 상단에 이미 있을 수 있으니 중복되면 정리한다. `everything_client.py`의 `json` import는 Task 4(기본 캘린더)에서 이미 추가됨 — 중복 import 금지.
