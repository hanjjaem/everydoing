# 파일명 검색 (칸 내 필터링) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 캘린더에서 파일명으로 검색해 매칭되는 파일만 날짜 칸에 남도록 클라이언트에서 필터링한다.

**Architecture:** 순수 프런트엔드 기능. 이미 메모리에 로드된 `data`를 검색어로 필터링한다. 필터 로직은 순수 함수 `filterFiles`로 분리하고, `render()`가 칸을 그릴 때 이를 거쳐 그린다. 서버 변경 없음.

**Tech Stack:** 바닐라 JS, HTML. (테스트는 Node로 순수 함수 실행 검증.)

## Global Constraints

- 서버 변경 없음 — `static/app.js`와 `static/index.html`만 수정.
- 검색은 대소문자 무시 부분 일치(파일명에 검색어 포함).
- 빈 검색어 = 전체 표시. 서버 재요청 없이 메모리 `data`만 사용.
- 검색창은 꾸밈 없는 기본 마크업(디자인은 추후 일괄 적용).
- 기존 동작(달력 렌더, 월 이동, 배지 펼침, 파일 메뉴, 설정 패널, 새로고침)은 그대로 유지.

---

## File Structure

- `static/index.html` — 헤더에 검색 입력칸 추가.
- `static/app.js` — `filterFiles` 순수 함수 추가, `render()`가 그것을 사용, 검색 입력 배선.

---

### Task 1: filterFiles 순수 함수 + render 연동 + 검색창

**Files:**
- Modify: `static/index.html`
- Modify: `static/app.js`

**Interfaces:**
- Consumes: 기존 전역 `data`(`{ "YYYY-MM-DD": [{name, path}] }`), 기존 `render()`, 기존 `key(y,m,d)`.
- Produces: `filterFiles(files, query)` — files 배열에서 `name`에 `query`(소문자)가 포함된 항목만 반환. `query`가 빈 문자열/공백이면 files 원본을 그대로 반환. 전역 `query` 문자열 상태.

- [ ] **Step 1: index.html — 헤더에 검색창 추가**

헤더의 `<button id="settings">폴더 설정</button>` 줄 바로 다음에 추가:

```html
    <input id="search" type="text" placeholder="파일명 검색">
```

- [ ] **Step 2: app.js — filterFiles 순수 함수와 query 상태 추가**

`app.js` 상단의 기존 `let data = {};` / `let cur = new Date();` 부근(상태 선언부)에 `query` 상태를 추가하고, 파일 상단(또는 render 위)에 `filterFiles`를 정의:

```javascript
let query = "";

function filterFiles(files, q) {
  const norm = (q || "").trim().toLowerCase();
  if (!norm) return files;
  return files.filter(f => (f.name || "").toLowerCase().includes(norm));
}
```

- [ ] **Step 3: app.js — render()가 filterFiles를 거치도록 수정**

`render()` 안에서 그날 파일을 가져오는 기존 줄:

```javascript
    const files = data[key(y, m, d)] || [];
```

를 다음으로 바꾼다:

```javascript
    const files = filterFiles(data[key(y, m, d)] || [], query);
```

(이후 `if (files.length) { ... 배지 = files.length + "개" ... files.forEach(...) }` 로직은 그대로 — 필터된 배열을 쓰므로 배지 수·펼친 목록이 자동으로 매칭만 반영된다.)

- [ ] **Step 4: app.js — 검색 입력 배선**

기존 이벤트 핸들러 등록부(예: `document.getElementById("refresh").onclick = load;` 부근)에 추가:

```javascript
document.getElementById("search").oninput = (e) => { query = e.target.value; render(); };
```

- [ ] **Step 5: filterFiles 동작 검증 (Node)**

`static/app.js`에서 `filterFiles` 함수 본문을 복사해 임시 파일 `_t.js`로 검증한다 (브라우저 전역에 의존하지 않는 순수 함수이므로 단독 실행 가능):

```bash
cat > _t.js <<'EOF'
function filterFiles(files, q) {
  const norm = (q || "").trim().toLowerCase();
  if (!norm) return files;
  return files.filter(f => (f.name || "").toLowerCase().includes(norm));
}
const data = [{name:"보고서.hwp"},{name:"REPORT.pdf"},{name:"메모.txt"}];
const eq = (a,b)=>JSON.stringify(a)===JSON.stringify(b);
console.assert(eq(filterFiles(data,""), data), "empty query = all");
console.assert(eq(filterFiles(data,"  "), data), "whitespace query = all");
console.assert(eq(filterFiles(data,"report").map(f=>f.name), ["REPORT.pdf"]), "case-insensitive");
console.assert(eq(filterFiles(data,"보고").map(f=>f.name), ["보고서.hwp"]), "substring korean");
console.assert(eq(filterFiles(data,"zzz"), []), "no match = empty");
console.log("filterFiles OK");
EOF
node _t.js && rm -f _t.js
```
Expected: `filterFiles OK` (no assertion failures).
Also confirm app.js parses: `node --check static/app.js` → no error.
Also confirm existing unit tests unaffected: `python -m pytest tests/ -q` → 18 passed.

- [ ] **Step 6: Commit**

```bash
git add static/index.html static/app.js
git commit -m "feat: filename search with in-cell filtering"
```

---

## Self-Review

**Spec coverage:**
- 헤더 검색 입력칸 → Step 1. ✓
- query 상태 + 입력 시 render 재호출(서버 요청 없음) → Step 2, 4. ✓
- 배지 개수가 매칭 수로 줄고 0이면 사라짐 → Step 3(필터된 배열을 기존 `if (files.length)`가 사용). ✓
- 칸 펼치면 매칭 파일만 → Step 3(같은 필터 배열을 forEach). ✓
- 빈 검색 = 전체 → filterFiles의 빈 쿼리 분기. ✓
- 대소문자 무시 부분 일치 → filterFiles(toLowerCase + includes). ✓
- 서버 변경 없음 → index.html/app.js만. ✓
- filterFiles 순수 함수 검증 → Step 5(Node). ✓

**Placeholder scan:** 없음. 모든 스텝에 실제 코드.

**Type consistency:** `filterFiles(files, q)` 정의(Step 2)와 사용(Step 3) 일치. 전역 `query` 선언(Step 2)·갱신(Step 4)·사용(Step 3) 일치. `data`/`key`/`render`는 기존 식별자 그대로, 충돌 없음.
