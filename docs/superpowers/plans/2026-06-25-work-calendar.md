# 업무정리캘린더 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 지정한 위치의 업무 문서를 수정일 기준으로 월 달력에 표시하는 개인용 로컬 웹을 만든다.

**Architecture:** 이미 설치된 Everything의 HTTP 서버를 인덱스 엔진으로 쿼리한다. Python 표준 라이브러리 로컬 서버가 Everything 결과를 받아 날짜별로 묶어 JSON으로 내려주고, 정적 캘린더 페이지(바닐라 HTML/JS)가 그걸 메모리에 들고 월 단위로 그린다. 파일 열기/폴더 열기는 서버가 수행한다.

**Tech Stack:** Python 3 표준 라이브러리만 (`http.server`, `urllib`, `json`, `datetime`, `os`, `subprocess`), 프런트엔드는 바닐라 HTML/CSS/JS. 외부 의존성 없음.

## Global Constraints

- 외부 파이썬 패키지 금지 — 표준 라이브러리만 사용한다.
- 플랫폼은 Windows 전용 (`os.startfile`, `explorer /select`).
- 인덱싱을 직접 만들지 않는다 — 파일 목록은 항상 Everything HTTP 서버를 통해 얻는다.
- 모든 설정은 `config.json` 한 곳에서 읽는다. 경로·포트·확장자를 코드에 하드코딩하지 않는다.
- 기준 날짜는 파일 **수정일(date_modified)** 이다.
- Everything 수정일은 Windows FILETIME(1601-01-01 기준 100ns 단위) 정수다. 일반 날짜로 변환해 사용한다.

---

## File Structure

- `config.json` — 설정(everything 호스트/포트, scan_paths, extensions, server_port). 구현 대상 코드 아님, 샘플 제공.
- `everything_client.py` — 순수 로직: config로 Everything 검색 쿼리 문자열 조립, HTTP 호출, JSON 파싱, FILETIME 변환, 날짜별 묶기. **네트워크 호출과 순수 변환을 함수로 분리**해 테스트 가능하게 한다.
- `server.py` — `http.server` 라우팅: 정적 파일 서빙, `GET /api/files`, `POST /api/open`.
- `static/index.html` — 캘린더 마크업 + CSS + 스크립트 로드.
- `static/app.js` — 캘린더 렌더링, 월 이동, 칸 펼침, 파일 클릭 메뉴, 새로고침.
- `tests/test_everything_client.py` — 쿼리 조립·FILETIME 변환·날짜별 묶기 단위 테스트.

---

### Task 1: Everything 쿼리 문자열 조립

**Files:**
- Create: `everything_client.py`
- Test: `tests/test_everything_client.py`

**Interfaces:**
- Consumes: (없음)
- Produces: `build_search_query(scan_paths: list[str], extensions: list[str]) -> str` — Everything 검색 문법 문자열을 반환한다. 확장자는 `ext:a;b;c`, 경로는 `("p1\" | "p2\")` 형태로 OR 결합. 둘을 공백으로 연결.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_everything_client.py
from everything_client import build_search_query


def test_build_search_query_combines_ext_and_paths():
    q = build_search_query(
        scan_paths=["C:\\Users\\u\\Desktop", "C:\\Users\\u\\Docs"],
        extensions=["hwp", "docx"],
    )
    assert q == 'ext:hwp;docx ("C:\\Users\\u\\Desktop\\" | "C:\\Users\\u\\Docs\\")'


def test_build_search_query_single_path():
    q = build_search_query(scan_paths=["C:\\A"], extensions=["pdf"])
    assert q == 'ext:pdf ("C:\\A\\")'
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest tests/test_everything_client.py::test_build_search_query_combines_ext_and_paths -v`
Expected: FAIL with `ImportError` / `cannot import name 'build_search_query'`

- [ ] **Step 3: Write minimal implementation**

```python
# everything_client.py

def build_search_query(scan_paths, extensions):
    ext_part = "ext:" + ";".join(extensions)
    quoted = ['"' + p.rstrip("\\") + '\\"' for p in scan_paths]
    path_part = "(" + " | ".join(quoted) + ")"
    return ext_part + " " + path_part
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest tests/test_everything_client.py -v`
Expected: PASS (2 passed)

- [ ] **Step 5: Commit**

```bash
git add everything_client.py tests/test_everything_client.py
git commit -m "feat: build Everything search query from config"
```

---

### Task 2: FILETIME → 날짜 변환

**Files:**
- Modify: `everything_client.py`
- Test: `tests/test_everything_client.py`

**Interfaces:**
- Consumes: (없음)
- Produces: `filetime_to_date(filetime: str | int) -> str | None` — Windows FILETIME을 `"YYYY-MM-DD"` 문자열로 변환. 비었거나 변환 불가면 `None`.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_everything_client.py 에 추가
from everything_client import filetime_to_date


def test_filetime_to_date_known_value():
    # 2026-06-12 00:00:00 UTC == FILETIME 133927776000000000
    assert filetime_to_date(133927776000000000) == "2026-06-12"


def test_filetime_to_date_accepts_string():
    assert filetime_to_date("133927776000000000") == "2026-06-12"


def test_filetime_to_date_empty_returns_none():
    assert filetime_to_date("") is None
    assert filetime_to_date(0) is None
    assert filetime_to_date("notanumber") is None
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest tests/test_everything_client.py::test_filetime_to_date_known_value -v`
Expected: FAIL with `cannot import name 'filetime_to_date'`

- [ ] **Step 3: Write minimal implementation**

```python
# everything_client.py 에 추가
from datetime import datetime, timedelta

_FILETIME_EPOCH = datetime(1601, 1, 1)


def filetime_to_date(filetime):
    try:
        ticks = int(filetime)
    except (TypeError, ValueError):
        return None
    if ticks <= 0:
        return None
    dt = _FILETIME_EPOCH + timedelta(microseconds=ticks / 10)
    return dt.strftime("%Y-%m-%d")
```

> 참고: 알려진 값 검증이 로컬 타임존에 흔들리지 않도록 변환은 UTC 기준 그대로 사용한다. (테스트 값은 UTC 자정 기준으로 계산됨.)

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest tests/test_everything_client.py -v`
Expected: PASS (5 passed)

- [ ] **Step 5: Commit**

```bash
git add everything_client.py tests/test_everything_client.py
git commit -m "feat: convert Windows FILETIME to date string"
```

---

### Task 3: Everything 결과 JSON → 날짜별 묶기

**Files:**
- Modify: `everything_client.py`
- Test: `tests/test_everything_client.py`

**Interfaces:**
- Consumes: `filetime_to_date`
- Produces: `group_by_date(results: list[dict]) -> dict[str, list[dict]]` — Everything 결과 리스트(각 항목은 `name`, `path`, `date_modified` 키)를 받아 `{ "YYYY-MM-DD": [{"name":..., "path":...}, ...] }` 로 묶는다. 변환 불가 날짜는 키 `"unknown"` 아래에 모은다.

- [ ] **Step 1: Write the failing test**

```python
# tests/test_everything_client.py 에 추가
from everything_client import group_by_date


def test_group_by_date_groups_and_keeps_name_path():
    results = [
        {"name": "a.hwp", "path": "C:\\x\\a.hwp", "date_modified": 133927776000000000},
        {"name": "b.docx", "path": "C:\\x\\b.docx", "date_modified": 133927776000000000},
    ]
    grouped = group_by_date(results)
    assert grouped == {
        "2026-06-12": [
            {"name": "a.hwp", "path": "C:\\x\\a.hwp"},
            {"name": "b.docx", "path": "C:\\x\\b.docx"},
        ]
    }


def test_group_by_date_unknown_bucket():
    results = [{"name": "c.pdf", "path": "C:\\c.pdf", "date_modified": ""}]
    grouped = group_by_date(results)
    assert grouped == {"unknown": [{"name": "c.pdf", "path": "C:\\c.pdf"}]}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python -m pytest tests/test_everything_client.py::test_group_by_date_groups_and_keeps_name_path -v`
Expected: FAIL with `cannot import name 'group_by_date'`

- [ ] **Step 3: Write minimal implementation**

```python
# everything_client.py 에 추가

def group_by_date(results):
    grouped = {}
    for item in results:
        date = filetime_to_date(item.get("date_modified")) or "unknown"
        grouped.setdefault(date, []).append(
            {"name": item.get("name"), "path": item.get("path")}
        )
    return grouped
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python -m pytest tests/test_everything_client.py -v`
Expected: PASS (7 passed)

- [ ] **Step 5: Commit**

```bash
git add everything_client.py tests/test_everything_client.py
git commit -m "feat: group Everything results by modified date"
```

---

### Task 4: Everything HTTP 호출 + 설정 로드

**Files:**
- Modify: `everything_client.py`
- Create: `config.json`

**Interfaces:**
- Consumes: `build_search_query`, `group_by_date`
- Produces:
  - `load_config(path="config.json") -> dict`
  - `fetch_files(config: dict) -> dict[str, list[dict]]` — config로 쿼리를 만들어 Everything HTTP 서버에 요청하고, 결과를 `group_by_date`로 묶어 반환한다. 연결 실패 시 `EverythingConnectionError`를 raise.
  - `class EverythingConnectionError(Exception)`

이 태스크는 실제 네트워크 I/O라 자동 단위 테스트 대신 수동 검증으로 확인한다(아래 Step에 절차 포함).

- [ ] **Step 1: config.json 샘플 작성**

```json
{
  "everything_host": "localhost",
  "everything_port": 8080,
  "scan_paths": [
    "C:\\Users\\user\\Desktop",
    "C:\\Users\\user\\Documents",
    "C:\\Users\\user\\Downloads"
  ],
  "extensions": ["hwp", "hwpx", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "pdf"],
  "server_port": 5000
}
```

- [ ] **Step 2: 구현 작성**

```python
# everything_client.py 에 추가
import json
import urllib.parse
import urllib.request


class EverythingConnectionError(Exception):
    pass


def load_config(path="config.json"):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def fetch_files(config):
    query = build_search_query(config["scan_paths"], config["extensions"])
    params = urllib.parse.urlencode({
        "search": query,
        "json": 1,
        "path_column": 1,
        "date_modified_column": 1,
    })
    url = "http://{host}:{port}/?{params}".format(
        host=config["everything_host"],
        port=config["everything_port"],
        params=params,
    )
    try:
        with urllib.request.urlopen(url, timeout=10) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except (urllib.error.URLError, OSError) as e:
        raise EverythingConnectionError(str(e))
    return group_by_date(data.get("results", []))
```

- [ ] **Step 3: 수동 검증 (사용자 확인)**

전제: Everything → 도구 → 옵션 → HTTP 서버를 켜고 포트를 config와 맞춘다.
Run: `python -c "import everything_client as e; print(list(e.fetch_files(e.load_config()).items())[:2])"`
Expected: 날짜별 파일 묶음 일부가 출력된다. (HTTP 서버가 꺼져 있으면 `EverythingConnectionError`)

- [ ] **Step 4: Commit**

```bash
git add everything_client.py config.json
git commit -m "feat: fetch files from Everything HTTP server"
```

---

### Task 5: 로컬 서버 라우팅 (정적 + /api/files + /api/open)

**Files:**
- Create: `server.py`

**Interfaces:**
- Consumes: `load_config`, `fetch_files`, `EverythingConnectionError`
- Produces: 실행 진입점 `python server.py` → `config["server_port"]`에서 서버 기동.
  - `GET /` 및 정적 파일 → `static/` 디렉터리 서빙
  - `GET /api/files` → `{ "YYYY-MM-DD": [...] }` JSON. 연결 실패 시 `503` + `{"error": "..."}`
  - `POST /api/open` → body `{"path": "...", "mode": "file"|"folder"}` 처리 후 `{"ok": true}` 또는 `404` + `{"error":"파일을 찾을 수 없습니다"}`

이 태스크는 통합 동작이라 수동 검증으로 확인한다.

- [ ] **Step 1: 구현 작성**

```python
# server.py
import json
import os
import subprocess
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse

import everything_client as ec

CONFIG = ec.load_config()
STATIC_DIR = os.path.join(os.path.dirname(__file__), "static")


class Handler(BaseHTTPRequestHandler):
    def _send_json(self, code, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = urlparse(self.path).path
        if path == "/api/files":
            try:
                self._send_json(200, ec.fetch_files(CONFIG))
            except ec.EverythingConnectionError as e:
                self._send_json(503, {"error": str(e)})
            return
        self._serve_static(path)

    def _serve_static(self, path):
        if path == "/":
            path = "/index.html"
        full = os.path.normpath(os.path.join(STATIC_DIR, path.lstrip("/")))
        if not full.startswith(STATIC_DIR) or not os.path.isfile(full):
            self.send_error(404)
            return
        ctype = "text/html" if full.endswith(".html") else \
            "application/javascript" if full.endswith(".js") else "text/plain"
        with open(full, "rb") as f:
            body = f.read()
        self.send_response(200)
        self.send_header("Content-Type", ctype + "; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        if urlparse(self.path).path != "/api/open":
            self.send_error(404)
            return
        length = int(self.headers.get("Content-Length", 0))
        data = json.loads(self.rfile.read(length).decode("utf-8"))
        target = data.get("path", "")
        if not os.path.exists(target):
            self._send_json(404, {"error": "파일을 찾을 수 없습니다"})
            return
        if data.get("mode") == "folder":
            subprocess.Popen(["explorer", "/select,", target])
        else:
            os.startfile(target)
        self._send_json(200, {"ok": True})


if __name__ == "__main__":
    port = CONFIG.get("server_port", 5000)
    print("http://localhost:{}".format(port))
    ThreadingHTTPServer(("localhost", port), Handler).serve_forever()
```

- [ ] **Step 2: 수동 검증 (사용자 확인)**

Run: `python server.py` 후 브라우저에서 `http://localhost:5000/api/files` 접속.
Expected: 날짜별 JSON이 보인다. (static/index.html은 아직 없으니 `GET /`는 404 — Task 6에서 추가)

- [ ] **Step 3: Commit**

```bash
git add server.py
git commit -m "feat: local server routing for files and open actions"
```

---

### Task 6: 캘린더 UI

**Files:**
- Create: `static/index.html`
- Create: `static/app.js`

**Interfaces:**
- Consumes: `GET /api/files`, `POST /api/open`
- Produces: 브라우저 화면. 시작 시 `/api/files`로 전체 데이터를 받아 메모리에 보관하고, 현재 월을 그린다. 월 이동은 재요청 없이 즉시. 칸 클릭 시 펼침. 파일 클릭 시 열기/폴더 열기/경로 복사 메뉴.

이 태스크는 UI라 수동 검증으로 확인한다.

- [ ] **Step 1: index.html 작성**

```html
<!doctype html>
<html lang="ko">
<head>
<meta charset="utf-8">
<title>업무정리캘린더</title>
<style>
  body { font-family: "Malgun Gothic", sans-serif; margin: 20px; }
  header { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }
  #grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 4px; }
  .cell { border: 1px solid #ddd; min-height: 72px; padding: 4px; font-size: 12px; }
  .cell.today { outline: 2px solid #3b82f6; }
  .daynum { color: #888; }
  .badge { background: #3b82f6; color: #fff; border-radius: 8px; padding: 0 6px; cursor: pointer; }
  .files { margin-top: 4px; }
  .file { cursor: pointer; color: #1d4ed8; text-decoration: underline; display: block; }
  #msg { color: #b91c1c; }
</style>
</head>
<body>
  <header>
    <button id="prev">◀</button>
    <strong id="label"></strong>
    <button id="next">▶</button>
    <button id="refresh">새로고침</button>
  </header>
  <div id="msg"></div>
  <div id="weekhead" style="display:grid;grid-template-columns:repeat(7,1fr);gap:4px;font-weight:bold;">
    <div>일</div><div>월</div><div>화</div><div>수</div><div>목</div><div>금</div><div>토</div>
  </div>
  <div id="grid"></div>
  <script src="app.js"></script>
</body>
</html>
```

- [ ] **Step 2: app.js 작성**

```javascript
let data = {};
let cur = new Date();

async function load() {
  const msg = document.getElementById("msg");
  msg.textContent = "";
  try {
    const res = await fetch("/api/files");
    if (!res.ok) {
      const err = await res.json();
      msg.textContent = "Everything HTTP 서버에 연결할 수 없습니다. " +
        "Everything → 도구 → 옵션 → HTTP 서버를 켰는지, 포트가 config.json과 맞는지 확인하세요. (" + err.error + ")";
      data = {};
    } else {
      data = await res.json();
    }
  } catch (e) {
    msg.textContent = "서버 응답을 받을 수 없습니다: " + e;
    data = {};
  }
  render();
}

function key(y, m, d) {
  return y + "-" + String(m + 1).padStart(2, "0") + "-" + String(d).padStart(2, "0");
}

function render() {
  const y = cur.getFullYear(), m = cur.getMonth();
  document.getElementById("label").textContent = y + "년 " + (m + 1) + "월";
  const grid = document.getElementById("grid");
  grid.innerHTML = "";
  const first = new Date(y, m, 1).getDay();
  const days = new Date(y, m + 1, 0).getDate();
  const today = new Date();
  for (let i = 0; i < first; i++) grid.appendChild(document.createElement("div"));
  for (let d = 1; d <= days; d++) {
    const cell = document.createElement("div");
    cell.className = "cell";
    if (y === today.getFullYear() && m === today.getMonth() && d === today.getDate())
      cell.classList.add("today");
    const num = document.createElement("div");
    num.className = "daynum";
    num.textContent = d;
    cell.appendChild(num);
    const files = data[key(y, m, d)] || [];
    if (files.length) {
      const badge = document.createElement("span");
      badge.className = "badge";
      badge.textContent = files.length + "개";
      const box = document.createElement("div");
      box.className = "files";
      box.style.display = "none";
      badge.onclick = () => { box.style.display = box.style.display === "none" ? "block" : "none"; };
      files.forEach(f => {
        const a = document.createElement("span");
        a.className = "file";
        a.textContent = f.name;
        a.onclick = () => menu(f);
        box.appendChild(a);
      });
      cell.appendChild(badge);
      cell.appendChild(box);
    }
    grid.appendChild(cell);
  }
}

function menu(f) {
  const choice = prompt(f.name + "\n1: 파일 열기  2: 폴더 열기  3: 경로 복사\n번호 입력:", "1");
  if (choice === "1") open(f.path, "file");
  else if (choice === "2") open(f.path, "folder");
  else if (choice === "3") navigator.clipboard.writeText(f.path);
}

async function open(path, mode) {
  const res = await fetch("/api/open", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path, mode }),
  });
  if (!res.ok) alert("파일을 찾을 수 없습니다");
}

document.getElementById("prev").onclick = () => { cur.setMonth(cur.getMonth() - 1); render(); };
document.getElementById("next").onclick = () => { cur.setMonth(cur.getMonth() + 1); render(); };
document.getElementById("refresh").onclick = load;
load();
```

- [ ] **Step 3: 수동 검증 (사용자 확인)**

Run: `python server.py` 후 브라우저에서 `http://localhost:5000/` 접속.
Expected:
1. 현재 월 달력이 뜨고, 파일 있는 날에 "N개" 배지가 보인다.
2. 배지 클릭 → 파일 목록 펼침/접힘.
3. 파일명 클릭 → 1/2/3 선택 → 파일 열기·폴더 열기·경로 복사 동작.
4. ◀ ▶ 로 월 이동 시 재요청 없이 즉시 그려진다.
5. Everything HTTP 서버를 끄고 새로고침 → 빨간 안내 메시지가 뜬다.

- [ ] **Step 4: Commit**

```bash
git add static/index.html static/app.js
git commit -m "feat: calendar UI with month nav and file actions"
```

---

## Self-Review

**Spec coverage:**
- Everything 인덱스 재활용 → Task 4. ✓
- Python 표준 라이브러리 서버 → Task 5. ✓
- 전체 한 번 로드 + 메모리 보관 + 월 이동 즉시 → Task 6 (`load()` 1회, `render()` 재요청 없음). ✓
- 수정일 기준 + FILETIME 변환 → Task 2. ✓
- 경로+확장자 필터 쿼리 → Task 1. ✓
- 날짜별 묶기 / unknown 버킷 → Task 3. ✓
- 개수 배지 + 클릭 펼침 → Task 6. ✓
- 파일 클릭 메뉴(파일열기/폴더열기/경로복사) → Task 5 (`/api/open`) + Task 6 (`menu`). ✓
- 새로고침 → Task 6 (`refresh` → `load`). ✓
- 에러 처리(연결 실패/0건/열기 실패/날짜 미상) → Task 4·5·6. (0건은 빈 달력으로 자연 표현)
- 검증 방법 → 각 통합 태스크의 수동 검증 Step. ✓
- 단위 테스트 범위(쿼리 조립·FILETIME·날짜 묶기) → Task 1·2·3. ✓

**Placeholder scan:** 없음. 모든 코드 스텝에 실제 코드 포함.

**Type consistency:** `build_search_query`, `filetime_to_date`, `group_by_date`, `fetch_files`, `load_config`, `EverythingConnectionError` 이름이 Task 간 일치. `/api/open` body 키(`path`, `mode`)가 server.py와 app.js에서 일치. `/api/files` 응답 형태(`{date: [{name,path}]}`)가 group_by_date·app.js에서 일치.
