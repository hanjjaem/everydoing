# Daily Memo Diary Cells Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 달력 칸을 "날짜 + 파일 수(숫자·점) + 내 한 줄 메모"를 담는 다이어리 칸으로 바꾸고, 히트맵 전체 칠을 제거한다.

**Architecture:** 메모는 `memos.json`에 날짜별 한 줄로 로컬 저장한다. `everything_client.py`에 순수 헬퍼(`set_memo`)와 로드/저장(`load_memos`/`save_memos`)을 추가하고, `server.py`에 `GET/POST /api/memos`를 연결한다. 프런트(`static/`)는 칸 안에서 숫자 클릭=파일 드로어, 여백 클릭=인라인 메모 편집으로 동작을 분리하고 히트맵 CSS를 제거한다.

**Tech Stack:** Python 3 표준 라이브러리(http.server), 바닐라 HTML/CSS/JS, pytest.

## Global Constraints

- 백엔드는 Python 표준 라이브러리만 사용(외부 패키지 금지).
- 외부 네트워크·AI 호출 금지. 완전 로컬.
- 데이터 파일(`memos.json`)은 `.gitignore`에 포함, 커밋 금지.
- Windows 전용 동작 유지(`os.startfile`, `explorer /select`).
- 설정은 `config.json` 한 곳, 코드 하드코딩 금지(메모는 별도 데이터 파일).
- 기존 코드 스타일 따름. 건드릴 것만 건드린다.

---

### Task 1: 메모 순수 헬퍼 + 로드/저장 (`everything_client.py`)

**Files:**
- Modify: `everything_client.py` (파일 끝에 함수 추가)
- Test: `tests/test_everything_client.py` (테스트 추가)

**Interfaces:**
- Produces:
  - `set_memo(memos: dict, date: str, text: str) -> dict` — 새 dict 반환(원본 불변). `text.strip()`이 빈 문자열이면 해당 `date` 키 삭제, 아니면 `strip()`된 값으로 설정.
  - `load_memos(path: str = "memos.json") -> dict` — 파일 없으면 `{}`.
  - `save_memos(memos: dict, path: str = "memos.json") -> None` — UTF-8, `ensure_ascii=False, indent=2`.

- [ ] **Step 1: 실패하는 테스트 작성**

`tests/test_everything_client.py` 상단 import에 추가:

```python
from everything_client import (
    set_memo,
    load_memos,
    save_memos,
)
```

파일 끝에 추가:

```python
def test_set_memo_adds_new():
    assert set_memo({}, "2026-06-25", "예산안 검토") == {"2026-06-25": "예산안 검토"}


def test_set_memo_overwrites():
    out = set_memo({"2026-06-25": "old"}, "2026-06-25", "new")
    assert out == {"2026-06-25": "new"}


def test_set_memo_strips_whitespace():
    assert set_memo({}, "2026-06-25", "  보고서  ") == {"2026-06-25": "보고서"}


def test_set_memo_blank_deletes_key():
    assert set_memo({"2026-06-25": "x"}, "2026-06-25", "   ") == {}


def test_set_memo_blank_missing_key_noop():
    assert set_memo({"2026-06-24": "y"}, "2026-06-25", "") == {"2026-06-24": "y"}


def test_set_memo_does_not_mutate_original():
    original = {"2026-06-25": "x"}
    set_memo(original, "2026-06-25", "z")
    assert original == {"2026-06-25": "x"}


def test_load_memos_missing_file_returns_empty():
    assert load_memos("does_not_exist_memos.json") == {}


def test_save_then_load_memos_roundtrip():
    memos = {"2026-06-25": "한글 메모", "2026-06-24": "두 번째"}
    fd, path = tempfile.mkstemp(suffix=".json")
    os.close(fd)
    try:
        save_memos(memos, path)
        assert load_memos(path) == memos
    finally:
        os.remove(path)
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `python -m pytest tests/test_everything_client.py -k memo -v`
Expected: FAIL — `ImportError`/`cannot import name 'set_memo'`

- [ ] **Step 3: 최소 구현**

`everything_client.py` 파일 끝에 추가:

```python
def set_memo(memos: dict, date: str, text: str) -> dict:
    """날짜별 메모를 갱신한 새 dict 반환. 빈 텍스트면 키 삭제. 원본 불변."""
    out = dict(memos)
    stripped = (text or "").strip()
    if stripped:
        out[date] = stripped
    else:
        out.pop(date, None)
    return out


def load_memos(path: str = "memos.json") -> dict:
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        return {}


def save_memos(memos: dict, path: str = "memos.json") -> None:
    with open(path, "w", encoding="utf-8") as f:
        json.dump(memos, f, ensure_ascii=False, indent=2)
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `python -m pytest tests/test_everything_client.py -k memo -v`
Expected: PASS (8 passed)

- [ ] **Step 5: 전체 테스트 확인 후 커밋**

Run: `python -m pytest tests/ -q` → 모두 통과

```bash
git add everything_client.py tests/test_everything_client.py
git commit -m "feat: memo helpers (set_memo, load_memos, save_memos)"
```

---

### Task 2: 메모 API 엔드포인트 (`server.py`)

**Files:**
- Modify: `server.py` (do_GET, do_POST 라우팅)
- Test: 수동(아래 curl)

**Interfaces:**
- Consumes: `ec.load_memos()`, `ec.save_memos()`, `ec.set_memo()` (Task 1)
- Produces:
  - `GET /api/memos` → `200 {<date>: <text>, ...}`
  - `POST /api/memos` body `{"date": "2026-06-25", "text": "..."}` → `200 {"ok": true}`. `date` 누락 시 `400`.

- [ ] **Step 1: GET 라우트 추가**

`server.py`의 `do_GET` 안, `/api/config` 블록 다음에 추가:

```python
        if path == "/api/memos":
            self._send_json(200, ec.load_memos())
            return
```

- [ ] **Step 2: POST 허용 라우트에 추가**

`do_POST`의 라우트 검사 수정:

```python
        if route not in ("/api/open", "/api/config", "/api/pick-folder", "/api/memos"):
            self.send_error(404)
            return
```

분기 디스패치(`else` 직전)에 추가:

```python
        elif route == "/api/memos":
            self._handle_save_memo(data)
```

- [ ] **Step 3: 핸들러 메서드 추가**

`_handle_pick_folder` 다음에 추가:

```python
    def _handle_save_memo(self, data):
        date = data.get("date")
        if not isinstance(date, str) or not date:
            self._send_json(400, {"error": "date 필요"})
            return
        text = data.get("text", "")
        try:
            memos = ec.set_memo(ec.load_memos(), date, text if isinstance(text, str) else "")
            ec.save_memos(memos)
        except OSError as e:
            self._send_json(500, {"error": str(e)})
            return
        self._send_json(200, {"ok": True})
```

- [ ] **Step 4: 수동 확인**

`python server.py` 실행 후 다른 터미널에서:

```bash
curl -X POST http://localhost:5000/api/memos -H "Content-Type: application/json" -d "{\"date\":\"2026-06-25\",\"text\":\"테스트 메모\"}"
curl http://localhost:5000/api/memos
```
Expected: 첫 호출 `{"ok": true}`, 둘째 호출에 `"2026-06-25": "테스트 메모"` 포함. 프로젝트 폴더에 `memos.json` 생성됨.

- [ ] **Step 5: gitignore에 memos.json 추가 후 커밋**

`.gitignore`의 "런타임/데이터 — 코드 아님" 블록에 `memos.json` 추가:

```
# 런타임/데이터 — 코드 아님
*.db
*.db-shm
*.db-wal
*.log
memos.json
```

Run: `git status` → `memos.json`이 목록에 없어야 함.

```bash
git add server.py .gitignore
git commit -m "feat: GET/POST /api/memos endpoints"
```

---

### Task 3: 히트맵 제거 + 칸 구조/스타일 (`static/index.html`)

**Files:**
- Modify: `static/index.html` (`<style>` 블록)

**Interfaces:**
- Produces: CSS 클래스 `.count`(우상단 숫자), `.dots`/`.dot`(밀도 점), `.memo`(본문 한 줄), `.memo-edit`(편집 입력). `data-heat` 및 `--heat-*` 제거.

- [ ] **Step 1: 히트맵 CSS 제거**

`:root`와 `:root[data-theme="dark"]`에서 `--heat-1/2/3` 세 줄씩 삭제.
아래 블록(있는 그대로) 삭제:

```css
  .cell[data-heat="1"] { background: var(--heat-1); }
  .cell[data-heat="2"] { background: var(--heat-2); }
  .cell[data-heat="3"] { background: var(--heat-3); }
  .cell.has-files:hover { filter: brightness(1.04); }
  :root[data-theme="dark"] .cell.has-files:hover { filter: brightness(1.2); }
```

그리고 daynum/count의 흰색 처리(`.cell[data-heat="2"] .daynum, .cell[data-heat="3"] .daynum { color: #fff; }`와 `.cell[data-heat="2"] .count, .cell[data-heat="3"] .count { ... }`) 삭제. `.has-files`의 hover/cursor는 남기되 배경 의존 제거:

`.cell.has-files { cursor: pointer; transition: filter .12s; }` →
`.cell.has-files { background: var(--card); }`

- [ ] **Step 2: 숫자·점·메모 스타일 추가**

`.daynum` 규칙들 다음에 추가(기존 `.count` 블록은 아래 내용으로 교체):

```css
  /* 우상단 파일 수 — 양에 따라 강조 (박스 없음) */
  .count {
    position: absolute; top: 9px; right: 10px;
    font-family: var(--serif); font-weight: 600; line-height: 1;
    color: var(--ink-3); cursor: pointer;
  }
  .count:hover { color: var(--accent); }
  .count[data-amt="1"] { font-size: 12px; color: var(--ink-2); }
  .count[data-amt="2"] { font-size: 14px; color: var(--ink); }
  .count[data-amt="3"] { font-size: 16px; color: var(--accent); font-weight: 700; }

  /* 밀도 점 — 문서 1개당 점 1개, 상한 표시 */
  .dots {
    position: absolute; top: 30px; right: 10px;
    display: flex; gap: 3px; align-items: center;
    color: var(--ink-3); font-size: 10px; line-height: 1;
  }
  .dot { width: 4px; height: 4px; border-radius: 50%; background: var(--ink-3); }

  /* 본문 한 줄 메모 */
  .memo {
    margin-top: auto; font-size: 12.5px; line-height: 1.45;
    color: var(--ink-2); cursor: text;
    overflow: hidden; display: -webkit-box;
    -webkit-line-clamp: 2; -webkit-box-orient: vertical;
  }
  .memo.empty { color: var(--ink-3); opacity: 0; transition: opacity .12s; }
  .cell:hover .memo.empty { opacity: .6; }
  .memo-edit {
    margin-top: auto; font: inherit; font-size: 12.5px; line-height: 1.45;
    width: 100%; resize: none; border: none; outline: none;
    background: transparent; color: var(--ink); padding: 0;
    font-family: var(--sans);
  }
```

- [ ] **Step 3: 브라우저에서 시각 확인(임시)**

이 단계 후 단독으로는 JS가 점/메모를 그리지 않으므로 다음 Task에서 확인. 여기서는 콘솔 에러만 없으면 됨.

- [ ] **Step 4: 커밋**

```bash
git add static/index.html
git commit -m "style: remove heatmap, add count/dots/memo cell styles"
```

---

### Task 4: 칸 렌더링·메모 편집·로드 (`static/app.js`)

**Files:**
- Modify: `static/app.js`

**Interfaces:**
- Consumes: `GET /api/memos`, `POST /api/memos` (Task 2); CSS 클래스 (Task 3).
- Produces: 전역 `memos` 객체; 순수 함수 `dotCount(n, cap=5) -> {dots:number, plus:boolean}`; 칸별 숫자 클릭=드로어, 여백 클릭=인라인 메모 편집.

- [ ] **Step 1: 전역 상태 + 순수 함수 추가**

`let selectedDate = null;` 다음 줄에 추가:

```javascript
let memos = {};
```

`heatLevel` 함수를 삭제하고 그 자리에 추가(히트맵 폐기):

```javascript
function amtLevel(n) {
  if (n <= 2) return 1;
  if (n <= 7) return 2;
  return 3;
}

function dotCount(n, cap = 5) {
  return { dots: Math.min(n, cap), plus: n > cap };
}
```

- [ ] **Step 2: `load()`에서 메모도 가져오기**

`load()` 안에서 `/api/files` 응답 처리 후 `renderAll()` 호출 직전에 메모 로드 추가. 기존:

```javascript
  } catch (e) {
    msg.textContent = "서버 응답을 받을 수 없습니다: " + e;
    data = {};
  }
  renderAll();
```

를 아래로 교체:

```javascript
  } catch (e) {
    msg.textContent = "서버 응답을 받을 수 없습니다: " + e;
    data = {};
  }
  try {
    const r = await fetch("/api/memos");
    memos = r.ok ? await r.json() : {};
  } catch (_) { memos = {}; }
  renderAll();
```

- [ ] **Step 3: `render()`의 셀 생성 교체**

`render()`에서 파일 있는 셀을 그리는 블록(현재 `.count` 생성 + `cell.onclick = selectDate` 부분)을 아래로 교체:

```javascript
    const files = filterFiles(data[cellKey] || [], query);
    if (files.length) {
      activeDays++; totalFiles += files.length;
      cell.classList.add("has-files");
      const count = document.createElement("span");
      count.className = "count";
      count.dataset.amt = amtLevel(files.length);
      count.textContent = files.length;
      count.onclick = (e) => { e.stopPropagation(); selectDate(cellKey); };
      cell.appendChild(count);

      const { dots, plus } = dotCount(files.length);
      const dotWrap = document.createElement("span");
      dotWrap.className = "dots";
      for (let i = 0; i < dots; i++) {
        const dot = document.createElement("span");
        dot.className = "dot";
        dotWrap.appendChild(dot);
      }
      if (plus) dotWrap.appendChild(document.createTextNode("+"));
      cell.appendChild(dotWrap);
    }
    attachMemo(cell, cellKey);
    grid.appendChild(cell);
```

- [ ] **Step 4: 메모 표시·편집 함수 추가**

`render()` 함수 다음(또는 `renderAll` 앞)에 추가:

```javascript
function attachMemo(cell, cellKey) {
  const memo = document.createElement("div");
  const text = memos[cellKey] || "";
  memo.className = text ? "memo" : "memo empty";
  memo.textContent = text || "메모 추가";
  memo.onclick = (e) => { e.stopPropagation(); startMemoEdit(cell, cellKey); };
  cell.appendChild(memo);
}

function startMemoEdit(cell, cellKey) {
  const existing = cell.querySelector(".memo");
  const ta = document.createElement("textarea");
  ta.className = "memo-edit";
  ta.value = memos[cellKey] || "";
  ta.rows = 2;
  if (existing) existing.replaceWith(ta); else cell.appendChild(ta);
  ta.focus();
  ta.setSelectionRange(ta.value.length, ta.value.length);
  const finish = (save) => {
    ta.onblur = null;
    if (save) saveMemo(cellKey, ta.value);
    else renderAll();
  };
  ta.onblur = () => finish(true);
  ta.onkeydown = (e) => {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); ta.blur(); }
    else if (e.key === "Escape") { e.preventDefault(); finish(false); }
  };
}

async function saveMemo(cellKey, text) {
  const trimmed = text.trim();
  if (trimmed) memos[cellKey] = trimmed; else delete memos[cellKey];
  render();
  try {
    await fetch("/api/memos", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ date: cellKey, text: trimmed }),
    });
  } catch (_) {}
}
```

- [ ] **Step 5: 수동 확인**

`python server.py` 후 브라우저 새로고침:
- 칸 여백 클릭 → textarea 등장 → 타이핑 → Enter/포커스 아웃 → 한 줄 표시.
- Esc → 저장 취소.
- 숫자 클릭 → 우측 드로어(파일 목록), 메모 편집과 충돌 없음.
- 점이 파일 수만큼(상한 5, 초과 시 `+`).
- 창 갔다 와도(focus) 메모 유지.
- 빈 메모로 저장 시 `memos.json`에서 키 사라짐.

- [ ] **Step 6: 커밋**

```bash
git add static/app.js
git commit -m "feat: diary cells - count/dots + inline memo editing"
```

---

## Self-Review 결과

- **Spec 커버리지:** 칸 구조(숫자/점/메모)=Task 3·4, 두 클릭 분리=Task 4 Step 3·4, 히트맵 제거=Task 3·4 Step 1, 메모 로컬 저장=Task 1·2, gitignore=Task 2 Step 5, 테스트=Task 1. 모두 매핑됨.
- **플레이스홀더:** 없음(모든 코드 블록 실제 내용).
- **타입 일관성:** `set_memo/load_memos/save_memos` 시그니처가 Task 1 정의와 Task 2 사용처 일치. `dotCount`는 `{dots, plus}` 반환을 Task 4 Step 1·3에서 동일하게 사용. `amtLevel`이 `heatLevel`을 대체하며 `data-amt` 1~3과 CSS(Task 3)의 `[data-amt]` 일치.
- **주의:** 기존 `heatLevel`은 Task 4 Step 1에서 삭제되므로 잔존 참조 없음(`render`의 `data-heat` 부여도 Step 3 교체로 제거).
