# 최근 작업 타임라인 중심 재설계 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 달력 옆에 "최근 작업" 타임라인을 주역으로 두는 2단 레이아웃으로 재설계하여, 열자마자 최근 한 일이 파일명으로 바로 읽히게 한다.

**Architecture:** 서버 변경 없이 `static/index.html`(2단 레이아웃 + 타임라인 마크업)과 `static/app.js`(상태 두 값 `cur`/`selectedDate`, 타임라인 렌더·달력 연동·hover 액션·빈 상태)만 수정한다. 타임라인 데이터 가공은 순수 함수로 분리해 Node로 테스트한다.

**Tech Stack:** 바닐라 HTML/CSS/JS. 서버·pytest 영향 없음.

## Global Constraints

- 서버 변경 없음 — `static/` 만 수정. 데이터는 기존 `GET /api/files`(`{ "YYYY-MM-DD": [{name, path}] }`).
- 상태는 두 값만: `cur`(현재 월 Date), `selectedDate`(선택 날짜 "YYYY-MM-DD" 또는 null). 달력·타임라인·요약이 이 둘로 그려진다.
- 기존 비주얼(종이 톤, 명조 마스트헤드, 단일 강조색 버건디/다크 테라코타, 다크 토글) 계승.
- 기존 기능 보존: 월 이동, 검색(filterFiles), 설정 패널(폴더·확장자), 다크 토글, 파일/폴더 열기·경로복사.
- 기존 `prompt()` 방식 파일 메뉴는 제거하고 타임라인 hover 액션으로 대체.
- 모달·페이지 전환 없음. 같은 화면 안에서 시선만 이동.
- 강조색·여백 위계는 크기·무게로. 타임라인은 카드로 감싸지 않음(여백 구분).

---

## File Structure

- `static/index.html` — 2단 레이아웃 컨테이너(좌 달력 `#cal`, 우 타임라인 `#timeline`), 타임라인 빈상태 요소, 관련 CSS. 마스트헤드·설정패널은 유지.
- `static/app.js` — 순수 함수(`formatDayLabel`, `buildGroups`), `renderTimeline`, `render`(달력) 리팩터, 상태 `selectedDate`, 연동·hover 액션·`openFile` 유지, `menu()` 제거.

---

### Task 1: 순수 함수 — 날짜 라벨 + 타임라인 그룹 생성

**Files:**
- Modify: `static/app.js`

**Interfaces:**
- Consumes: 기존 `filterFiles(files, q)`, 기존 `key(y,m,d)`.
- Produces:
  - `formatDayLabel(dateStr, today)` — `dateStr`="YYYY-MM-DD", `today`=Date. 같은 날이면 `"오늘 · 6월 25일"`, 하루 전이면 `"어제 · 6월 24일"`, 그 외 `"6월 23일 · 월"`(요일). 반환 문자열.
  - `buildGroups(data, y, m, q)` — `data`={dateStr:[{name,path}]}, `y`/`m`(0-based 월), `q` 검색어. 해당 연·월에 속하는 날짜만, **날짜 내림차순** 배열로: `[{ date:"YYYY-MM-DD", files:[{name,path}] }, ...]`. 각 그룹 files는 `filterFiles`로 거른 결과이며, 거른 뒤 빈 그룹은 제외.

- [ ] **Step 1: app.js 상단(기존 `filterFiles` 정의 아래)에 순수 함수 추가**

```javascript
const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];

function formatDayLabel(dateStr, today) {
  const [y, m, d] = dateStr.split("-").map(Number);
  const that = new Date(y, m - 1, d);
  const base = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const diffDays = Math.round((base - that) / 86400000);
  const md = (m) + "월 " + (d) + "일";
  if (diffDays === 0) return "오늘 · " + md;
  if (diffDays === 1) return "어제 · " + md;
  return md + " · " + WEEKDAYS[that.getDay()];
}

function buildGroups(data, y, m, q) {
  const prefix = y + "-" + String(m + 1).padStart(2, "0") + "-";
  return Object.keys(data)
    .filter(k => k.startsWith(prefix))
    .sort((a, b) => (a < b ? 1 : -1))
    .map(date => ({ date, files: filterFiles(data[date], q) }))
    .filter(g => g.files.length > 0);
}
```

- [ ] **Step 2: Node로 순수 함수 검증**

```bash
cat > _t.js <<'EOF'
const WEEKDAYS = ["일","월","화","수","목","금","토"];
function filterFiles(files,q){const n=(q||"").trim().toLowerCase();if(!n)return files;return files.filter(f=>(f.name||"").toLowerCase().includes(n));}
function formatDayLabel(dateStr, today){const[y,m,d]=dateStr.split("-").map(Number);const that=new Date(y,m-1,d);const base=new Date(today.getFullYear(),today.getMonth(),today.getDate());const diffDays=Math.round((base-that)/86400000);const md=(m)+"월 "+(d)+"일";if(diffDays===0)return"오늘 · "+md;if(diffDays===1)return"어제 · "+md;return md+" · "+WEEKDAYS[that.getDay()];}
function buildGroups(data,y,m,q){const prefix=y+"-"+String(m+1).padStart(2,"0")+"-";return Object.keys(data).filter(k=>k.startsWith(prefix)).sort((a,b)=>(a<b?1:-1)).map(date=>({date,files:filterFiles(data[date],q)})).filter(g=>g.files.length>0);}

const today=new Date(2026,5,25);
console.assert(formatDayLabel("2026-06-25",today)==="오늘 · 6월 25일","today");
console.assert(formatDayLabel("2026-06-24",today)==="어제 · 6월 24일","yesterday");
console.assert(formatDayLabel("2026-06-23",today)==="6월 23일 · 화","weekday "+formatDayLabel("2026-06-23",today));
const data={"2026-06-23":[{name:"a.hwp",path:"p"}],"2026-06-10":[{name:"b.pdf",path:"p"}],"2026-05-30":[{name:"old.doc",path:"p"}]};
const g=buildGroups(data,2026,5,"");
console.assert(g.length===2,"month filter len "+g.length);
console.assert(g[0].date==="2026-06-23","desc order");
const gq=buildGroups(data,2026,5,"pdf");
console.assert(gq.length===1&&gq[0].date==="2026-06-10","search filter");
console.log("OK");
EOF
node _t.js && rm -f _t.js
```
Expected: `OK` (no assertion output). 2026-06-23은 화요일이므로 라벨 검증 통과.

- [ ] **Step 3: Commit**

```bash
git add static/app.js
git commit -m "feat: timeline pure helpers (formatDayLabel, buildGroups)"
```

---

### Task 2: 2단 레이아웃 + 타임라인 스타일 (HTML/CSS)

**Files:**
- Modify: `static/index.html`

**Interfaces:**
- Consumes: 기존 비주얼 토큰(`--paper`,`--card`,`--ink`,`--ink-2`,`--ink-3`,`--line`,`--accent`,`--accent-soft`,`--serif`,`--sans` 등 — 현재 :root에 정의됨).
- Produces: `#layout`(2단 컨테이너) 안에 좌 `#cal`(기존 `#weekhead`+`#grid` 이동), 우 `#timeline`. 타임라인 항목 클래스: `.tl-group`, `.tl-date`, `.tl-file`, `.tl-name`, `.tl-actions`, `.tl-empty`.

- [ ] **Step 1: 본문 구조를 2단으로 변경**

기존 `#weekhead`와 `#grid` 두 블록을 `#layout` 컨테이너로 감싸고 우측에 타임라인을 추가한다. `index.html`의:
```html
  <div id="weekhead">
    <div>일</div><div>월</div><div>화</div><div>수</div><div>목</div><div>금</div><div>토</div>
  </div>
  <div id="grid"></div>
```
를 다음으로 교체:
```html
  <div id="layout">
    <section id="cal">
      <div id="weekhead">
        <div>일</div><div>월</div><div>화</div><div>수</div><div>목</div><div>금</div><div>토</div>
      </div>
      <div id="grid"></div>
    </section>
    <aside id="timeline"></aside>
  </div>
```

- [ ] **Step 2: CSS 추가 (기존 `<style>` 끝, `</style>` 직전에 추가)**

```css
  /* ── 2단 레이아웃 ───────────────────────── */
  #layout {
    display: grid; grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
    gap: clamp(20px, 3vw, 44px); align-items: start;
  }
  @media (max-width: 880px) { #layout { grid-template-columns: 1fr; } }

  /* ── 타임라인 ───────────────────────── */
  #timeline { display: flex; flex-direction: column; gap: 22px;
    max-height: 72vh; overflow-y: auto; padding-right: 4px; }
  .tl-group { display: flex; flex-direction: column; gap: 7px;
    border-radius: 8px; transition: background .35s; }
  .tl-group.flash { background: var(--accent-soft); }
  .tl-date {
    font-family: var(--serif); font-size: 14px; font-weight: 600; color: var(--ink);
    padding-bottom: 4px; border-bottom: 1px solid var(--line);
  }
  .tl-file {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    padding: 6px 8px; margin: 0 -8px; border-radius: 7px; cursor: pointer;
    transition: background .12s;
  }
  .tl-file:hover { background: var(--hover); }
  .tl-name {
    font-size: 13.5px; color: var(--ink); overflow: hidden;
    text-overflow: ellipsis; white-space: nowrap;
  }
  .tl-file:hover .tl-name { color: var(--accent); }
  .tl-actions { display: none; gap: 4px; flex-shrink: 0; }
  .tl-file:hover .tl-actions { display: flex; }
  .tl-actions button {
    font: inherit; font-size: 11px; color: var(--ink-2);
    background: transparent; border: 1px solid var(--line-2); border-radius: 5px;
    padding: 2px 7px; cursor: pointer; transition: background .12s, color .12s;
  }
  .tl-actions button:hover { background: var(--accent); color: #fff; border-color: var(--accent); }
  .tl-empty { color: var(--ink-3); font-size: 13.5px; line-height: 1.7; }
  .tl-empty button {
    display: inline-block; margin-top: 8px; font: inherit; font-size: 13px;
    background: var(--accent); color: #fff; border: none; border-radius: 6px;
    padding: 7px 13px; cursor: pointer;
  }
  /* 달력에서 선택된 날 표시 */
  .cell.selected { box-shadow: inset 0 0 0 2px var(--accent); }
```

- [ ] **Step 3: 구문 확인 + 회귀**

Run: `python -m pytest tests/ -q`
Expected: 18 passed (서버 무관).
(외관은 Task 3 완료 후 브라우저에서 확인.)

- [ ] **Step 4: Commit**

```bash
git add static/index.html
git commit -m "feat: two-column layout and timeline styles"
```

---

### Task 3: 타임라인 렌더 + 달력 연동 + 인터랙션 (app.js)

**Files:**
- Modify: `static/app.js`

**Interfaces:**
- Consumes: Task 1의 `formatDayLabel`, `buildGroups`; 기존 `data`, `cur`, `query`, `render`, `key`, `openFile`, `load`.
- Produces: 전역 `selectedDate`(null|string), 함수 `renderTimeline()`, `renderAll()`(달력+타임라인+요약 일괄). 기존 `menu()` 제거. 달력 배지/날짜 클릭이 `selectDate(dateStr)` 호출.

- [ ] **Step 1: 상태 추가 + render 분리**

기존 `let query = "";` 아래에 추가:
```javascript
let selectedDate = null;
```
기존 `render()` 함수의 끝(요약 세팅까지) 뒤에, 달력+타임라인을 함께 그리는 래퍼와 타임라인 렌더를 추가한다. `render()` 안 마지막의 요약 세팅은 그대로 두고, **새 함수**를 추가:

```javascript
function renderAll() {
  render();          // 달력 + 요약
  renderTimeline();  // 우측 타임라인
}

function renderTimeline() {
  const y = cur.getFullYear(), m = cur.getMonth();
  const tl = document.getElementById("timeline");
  tl.innerHTML = "";
  const groups = buildGroups(data, y, m, query);
  if (!groups.length) {
    const empty = document.createElement("div");
    empty.className = "tl-empty";
    empty.innerHTML = query
      ? "검색 결과가 없어요."
      : "아직 기록이 없어요.<br>설정에서 작업 폴더를 추가해 보세요.";
    if (!query) {
      const b = document.createElement("button");
      b.textContent = "설정 열기";
      b.onclick = openPanel;
      empty.appendChild(document.createElement("br"));
      empty.appendChild(b);
    }
    tl.appendChild(empty);
    return;
  }
  const today = new Date();
  groups.forEach(g => {
    const grp = document.createElement("div");
    grp.className = "tl-group";
    grp.dataset.date = g.date;
    const h = document.createElement("div");
    h.className = "tl-date";
    h.textContent = formatDayLabel(g.date, today);
    grp.appendChild(h);
    g.files.forEach(f => {
      const row = document.createElement("div");
      row.className = "tl-file";
      const name = document.createElement("span");
      name.className = "tl-name";
      name.textContent = f.name;
      row.appendChild(name);
      const acts = document.createElement("span");
      acts.className = "tl-actions";
      acts.appendChild(actionBtn("열기", e => { e.stopPropagation(); openFile(f.path, "file"); }));
      acts.appendChild(actionBtn("폴더", e => { e.stopPropagation(); openFile(f.path, "folder"); }));
      acts.appendChild(actionBtn("경로", e => { e.stopPropagation(); navigator.clipboard.writeText(f.path); }));
      row.appendChild(acts);
      row.onclick = () => openFile(f.path, "file");
      grp.appendChild(row);
    });
    tl.appendChild(grp);
  });
}

function actionBtn(label, handler) {
  const b = document.createElement("button");
  b.textContent = label;
  b.onclick = handler;
  return b;
}

function selectDate(dateStr) {
  selectedDate = (selectedDate === dateStr) ? null : dateStr;
  render();  // 달력 선택 표시 갱신
  if (selectedDate) {
    const grp = document.querySelector('#timeline .tl-group[data-date="' + selectedDate + '"]');
    if (grp) {
      grp.scrollIntoView({ behavior: "smooth", block: "start" });
      grp.classList.add("flash");
      setTimeout(() => grp.classList.remove("flash"), 700);
    }
  }
}
```

- [ ] **Step 2: 달력 칸에 선택 표시 + 클릭 연동, prompt 메뉴 제거**

`render()` 안에서 today 처리 부근에, 선택 표시와 클릭을 추가한다. 기존:
```javascript
    if (y === today.getFullYear() && m === today.getMonth() && d === today.getDate())
      cell.classList.add("today");
```
다음 줄에 추가:
```javascript
    const cellKey = key(y, m, d);
    if (cellKey === selectedDate) cell.classList.add("selected");
```
그리고 기존 배지 클릭(현재 펼침 토글)을 타임라인 점프로 바꾼다. `render()` 안 배지 생성부의:
```javascript
      badge.onclick = () => { box.style.display = box.style.display === "none" ? "block" : "none"; };
```
를:
```javascript
      badge.onclick = () => selectDate(cellKey);
```
또한 칸 안의 펼침 목록(`box`, `files.forEach(... a.onclick = () => menu(f) ...)`)은 **제거**한다(타임라인이 그 역할을 대신함). 즉 `render()`의 배지 블록에서 `box` 생성·파일 append·`cell.appendChild(box)` 부분을 삭제하고 배지만 추가한다. 결과적으로 배지 블록은 다음만 남는다:
```javascript
    if (files.length) {
      activeDays++; totalFiles += files.length;
      cell.classList.add("has-files");
      const badge = document.createElement("span");
      badge.className = "badge";
      badge.innerHTML = '<span class="n"></span><span class="u">개</span>';
      badge.querySelector(".n").textContent = files.length;
      badge.onclick = () => selectDate(cellKey);
      cell.appendChild(badge);
    }
```
그리고 기존 `menu(f)` 함수 정의를 **삭제**한다(더 이상 사용 안 함).

- [ ] **Step 3: 월 이동·검색·새로고침이 renderAll을 쓰도록 배선 교체**

기존 배선부에서 `render()`/`load` 호출을 타임라인까지 갱신하도록 바꾼다.
- 월 이동:
```javascript
document.getElementById("prev").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() - 1, 1); selectedDate = null; renderAll(); };
document.getElementById("next").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() + 1, 1); selectedDate = null; renderAll(); };
```
- 검색:
```javascript
document.getElementById("search").oninput = (e) => { query = e.target.value; renderAll(); };
```
- 마스트헤드 월(오늘 복귀): `#label`에 클릭 추가:
```javascript
document.getElementById("label").style.cursor = "pointer";
document.getElementById("label").title = "오늘로";
document.getElementById("label").onclick = () => { cur = new Date(); selectedDate = null; renderAll(); };
```
- `load()` 내부 마지막의 `render();` 호출을 `renderAll();`로 바꾼다(데이터 로드 후 타임라인도 그림). `savePaths` 성공 후의 `load()`는 그대로(내부에서 renderAll 호출됨).

- [ ] **Step 4: 검증**

Run: `node --check static/app.js`
Expected: 오류 없음.
Run: `python -m pytest tests/ -q`
Expected: 18 passed.

- [ ] **Step 5: Commit**

```bash
git add static/app.js
git commit -m "feat: render timeline, link calendar, hover actions, empty state"
```

---

## Self-Review

**Spec coverage:**
- 2단 레이아웃(좌 달력 / 우 타임라인) → Task 2. ✓
- 타임라인 주역·날짜 그룹 역순·파일명 노출 → Task 1(buildGroups), Task 3(renderTimeline). ✓
- 오늘/어제/요일 라벨 → Task 1(formatDayLabel). ✓
- hover 액션(열기·폴더·경로) + 행 클릭 열기, prompt 제거 → Task 3 Step 1,2. ✓
- 빈 상태(설정 유도) → Task 3 renderTimeline. ✓
- 검색 연동(타임라인+달력) → Task 3 Step 3(renderAll). ✓
- 상태 두 값(cur, selectedDate) → Task 3. ✓
- 날짜 클릭 → 타임라인 스크롤+강조, 달력 선택 표시 → Task 3(selectDate, .selected). ✓
- 월 이동 시 selectedDate 초기화 → Task 3 Step 3. ✓
- 오늘 복귀(마스트헤드) → Task 3 Step 3. ✓
- 모달 없음, 같은 화면 → 전부 인라인 렌더. ✓
- 비주얼 토큰 계승 → Task 2가 기존 변수 사용. ✓
- 반응형 1단 → Task 2 @media. ✓
- 서버 변경 없음 → static만. ✓
- 검증(Node 순수함수, pytest 18) → Task 1 Step 2, Task 3 Step 4. ✓

**Placeholder scan:** 없음. 모든 코드 스텝에 실제 코드.

**Type consistency:** `formatDayLabel(dateStr, today)`/`buildGroups(data,y,m,q)` 정의(Task 1)와 사용(Task 3) 일치. `selectDate`/`renderTimeline`/`renderAll`/`actionBtn` 정의·호출 일치. `cur`/`query`/`selectedDate`/`data` 전역 일관. `key(y,m,d)` 반환 "YYYY-MM-DD"가 buildGroups의 date 키·selectedDate 비교와 동일 포맷. CSS 클래스(`tl-group`/`tl-date`/`tl-file`/`tl-name`/`tl-actions`/`tl-empty`/`flash`/`selected`)가 Task 2 정의·Task 3 사용 일치. `#timeline`/`#layout`/`#cal` id 일치.

**Note:** Task 3 Step 2는 기존 `render()`의 배지 블록을 축소(펼침 box 제거)하고 `menu()`를 삭제하는 정밀 변경이다. 현재 app.js의 해당 블록을 정확히 찾아 교체할 것.
