const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

// The browser development bridge does not expose native window controls.
if (window.__TAURI__.window?.getCurrentWindow) {
  const appWindow = window.__TAURI__.window.getCurrentWindow();
  const maximizeButton = document.getElementById("window-maximize");
  document.documentElement.classList.add("desktop-shell");

  const refreshMaximizeButton = async () => {
    try {
      const maximized = await appWindow.isMaximized();
      maximizeButton.classList.toggle("is-maximized", maximized);
      maximizeButton.setAttribute("aria-label", maximized ? "이전 크기로" : "최대화");
      maximizeButton.title = maximized ? "이전 크기로" : "최대화";
    } catch (error) { console.error("창 상태를 확인하지 못했습니다:", error); }
  };
  document.getElementById("window-minimize").addEventListener("click", () => {
    appWindow.minimize().catch(error => console.error("창을 최소화하지 못했습니다:", error));
  });
  maximizeButton.addEventListener("click", async () => {
    try { await appWindow.toggleMaximize(); await refreshMaximizeButton(); }
    catch (error) { console.error("창 크기를 변경하지 못했습니다:", error); }
  });
  document.getElementById("window-close").addEventListener("click", () => {
    appWindow.close().catch(error => console.error("창을 닫지 못했습니다:", error));
  });
  window.addEventListener("resize", refreshMaximizeButton);
  refreshMaximizeButton();
}

let data = {};
let cur = new Date();
let query = "";
let selectedDate = null;
let memos = {};
let animateNext = true;
let scanPaths = [];
let monthStats = { activeDays: 0, totalFiles: 0, busyDate: null, busyCount: 0 };
let scanning = false;
let loaded = false;
let summaries = {};
let monthlySummaries = {};
let lastScan = 0;          // 마지막으로 스캔을 마친 시각(Unix 초). 0이면 모름.
let fileMenuPath = null;

function showFileMenu(e, path, fullName) {
  e.stopPropagation();
  fileMenuPath = path;
  const menu = document.getElementById("file-menu");
  document.getElementById("file-menu-name").textContent = fullName;
  menu.style.display = "flex";
  const r = e.currentTarget.getBoundingClientRect();
  let top = r.bottom + 4;
  let left = r.left;
  if (left + 120 > window.innerWidth) left = window.innerWidth - 124;
  if (top + 80 > window.innerHeight) top = r.top - 84;
  menu.style.top = top + "px";
  menu.style.left = left + "px";
}

function hideFileMenu() {
  document.getElementById("file-menu").style.display = "none";
  fileMenuPath = null;
}

const MONTHS = ["1월","2월","3월","4월","5월","6월","7월","8월","9월","10월","11월","12월"];
const LEGEND = [
  { color: "#1E6FE8", label: "HWP", name: "한글 (hwp)" },
  { color: "#2B579A", label: "DOC", name: "워드 (doc)" },
  { color: "#217346", label: "XLS", name: "엑셀 (xls)" },
  { color: "#C43E1C", label: "PPT", name: "PPT (ppt)" },
  { color: "#E5252A", label: "PDF", name: "PDF" },
];

// 파일명이나 폴더 경로에 검색어가 들어 있는지.
function matchesQuery(f, norm) {
  return (f.name || "").toLowerCase().includes(norm) || (f.path || "").toLowerCase().includes(norm);
}

function filterFiles(files, q) {
  const norm = (q || "").trim().toLowerCase();
  if (!norm) return files;
  return files.filter(f => matchesQuery(f, norm));
}

const SEARCH_LIMIT = 300;

// 전체 기간에서 검색어에 맞는 파일을 최근 날짜부터 모은다. 화면에는 limit개까지만 담는다.
function searchAll(data, q, limit) {
  const norm = (q || "").trim().toLowerCase();
  const groups = [];
  let total = 0, shown = 0;
  if (!norm) return { groups, total, shown };
  const dates = Object.keys(data).filter(k => /^\d{4}-\d{2}-\d{2}$/.test(k)).sort().reverse();
  for (const date of dates) {
    const hits = data[date].filter(f => matchesQuery(f, norm));
    if (!hits.length) continue;
    total += hits.length;
    if (shown >= limit) continue;
    const files = hits.slice(0, limit - shown);
    shown += files.length;
    groups.push({ date, files, count: hits.length });
  }
  return { groups, total, shown };
}

// 수정 시각(Unix 초) → "14:05". 시각을 모르면(옛 색인) 빈 문자열.
function timeOf(f) {
  if (!f.modified) return "";
  const d = new Date(f.modified * 1000);
  return String(d.getHours()).padStart(2, "0") + ":" + String(d.getMinutes()).padStart(2, "0");
}

// 파일이 들어 있는 폴더 경로.
function folderOf(path) {
  const i = Math.max((path || "").lastIndexOf("\\"), (path || "").lastIndexOf("/"));
  return i > 0 ? path.slice(0, i) : "";
}

const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];

const EXT_TYPE = {
  hwp:  { color: "#1E6FE8", label: "한" },
  hwpx: { color: "#1E6FE8", label: "한" },
  doc:  { color: "#2B579A", label: "W" },
  docx: { color: "#2B579A", label: "W" },
  xls:  { color: "#217346", label: "X" },
  xlsx: { color: "#217346", label: "X" },
  ppt:  { color: "#C43E1C", label: "P" },
  pptx: { color: "#C43E1C", label: "P" },
  pdf:  { color: "#E5252A", label: "F" },
};

function badgeFor(ext) {
  const MAP = { hwp:"HWP", hwpx:"HWP", doc:"DOC", docx:"DOC", xls:"XLS", xlsx:"XLS", ppt:"PPT", pptx:"PPT", pdf:"PDF" };
  return MAP[ext] || ext.slice(0, 3).toUpperCase();
}

function extOf(name) {
  const i = (name || "").lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}

function chipFor(ext) {
  return EXT_TYPE[ext] || { color: "#9b958a", label: (ext[0] || "?").toUpperCase() };
}

function typeChips(files, cap = 4) {
  const seen = [], out = [];
  for (const f of files) {
    const ext = extOf(f.name);
    const chip = chipFor(ext);
    const key = EXT_TYPE[ext] ? chip.label : ext;
    if (!seen.includes(key)) { seen.push(key); out.push(chip); }
  }
  return { chips: out.slice(0, cap), plus: out.length > cap };
}

function formatDayLabel(dateStr, today) {
  const [y, m, d] = dateStr.split("-").map(Number);
  const that = new Date(y, m - 1, d);
  const base = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  const diffDays = Math.round((base - that) / 86400000);
  const md = m + "월 " + d + "일 " + WEEKDAYS[that.getDay()];
  if (diffDays === 0) return "오늘 · " + md;
  if (diffDays === 1) return "어제 · " + md;
  return md;
}

function buildGroups(data, y, m, q) {
  const prefix = y + "-" + String(m + 1).padStart(2, "0") + "-";
  return Object.keys(data)
    .filter(k => k.startsWith(prefix))
    .sort((a, b) => (a < b ? 1 : -1))
    .map(date => ({ date, files: filterFiles(data[date], q) }))
    .filter(g => g.files.length > 0);
}

async function load() {
  if (loaded) return;
  loaded = true;
  console.log("[load] 시작");
  const msg = document.getElementById("msg");
  try { memos = await invoke("get_memos"); } catch (e) { memos = {}; alert("메모를 불러오지 못했습니다: " + e); }
  try { scanPaths = (await invoke("get_config")).scan_paths || []; }
  catch (e) { msg.textContent = "설정을 불러오지 못했습니다: " + e; return; }
  console.log("[load] scan_paths:", scanPaths);

  if (scanPaths.length === 0) {
    console.log("[load] scan_paths 없음 → 설정 패널 오픈");
    msg.textContent = "";
    data = {};
    renderAll();
    openPanel();
    return;
  }

  const cached = await invoke("load_cache").catch((e) => { console.error("[load] load_cache 오류:", e); return null; });
  console.log("[load] 캐시:", cached ? `있음 (날짜 ${Object.keys(cached.files||{}).length}개)` : "없음");
  if (cached) {
    data = cached.files || {};
    lastScan = cached.scanned_at || 0;
    summaries = await invoke("get_summaries").catch(e => { alert("일별 요약을 불러오지 못했습니다: " + e); return {}; });
    monthlySummaries = await invoke("get_monthly_summaries").catch(e => { alert("월간 요약을 불러오지 못했습니다: " + e); return {}; });
    msg.textContent = "";
    animateNext = true;
    renderAll();
    // 자동 생성하지 않음. AI 요약은 사용자가 '_AI 요약 생성_' 버튼을 눌렀을 때만 전송된다.
    // 저장된 기록을 먼저 보여 주고, 마지막 스캔 뒤에 바뀐 파일은 뒤에서 다시 스캔해 반영한다.
    await startScan({ quiet: true });
    return;
  }

  console.log("[load] 캐시 없음 → startScan 호출");
  await startScan();
}

// 메모나 요약을 입력하는 중인지. 입력 중에는 화면을 다시 그리지 않는다.
function isEditing() {
  const ae = document.activeElement;
  return !!ae && /^(memo-edit|day-summary-input|tl-summary-input)$/.test(ae.className || "");
}

// quiet: 저장된 기록을 보여 준 채 뒤에서 스캔한다(큰 진행 표시 없이 사이드바에만 표시).
async function startScan({ quiet = false } = {}) {
  if (scanning) return;
  scanning = true;
  const msg = document.getElementById("msg");
  const wrap = document.getElementById("progress-wrap");
  const label = document.getElementById("progress-label");
  const sub = document.getElementById("progress-sub");
  const elapsed = document.getElementById("progress-elapsed");

  msg.textContent = "";
  wrap.style.display = quiet ? "none" : "block";
  renderScanStatus();
  label.textContent = "스캔 준비 중...";
  sub.textContent = "";
  elapsed.textContent = "";
  const bar = document.getElementById("progress-bar");
  bar.style.animation = "";
  bar.style.width = "0%";
  bar.style.transition = "";
  bar.classList.add("pulse");

  const startTime = Date.now();
  const timer = setInterval(() => {
    const secs = Math.floor((Date.now() - startTime) / 1000);
    elapsed.textContent = `${secs}초째 진행 중`;
  }, 1000);

  let unlistenProgress = () => {};
  let unlistenDone = () => {};
  const finishScan = () => {
    clearInterval(timer);
    unlistenProgress();
    unlistenDone();
    scanning = false;
    wrap.style.display = "none";
    renderScanStatus();
  };

  try {
    unlistenProgress = await listen("scan-progress", (e) => {
      const { drive, scanned, total, matched } = e.payload;
      label.textContent = `${drive} 스캔 중`;
      sub.textContent = `파일 ${matched.toLocaleString()}개 발견`;
      if (total > 0) {
        bar.classList.remove("pulse");
        bar.style.transition = "width 0.4s ease";
        bar.style.width = Math.round((scanned / total) * 100) + "%";
      }
    });

    unlistenDone = await listen("scan-done", async (e) => {
      const result = e.payload;
      // 뒤에서 돈 재스캔이 스캔 폴더 자체를 읽지 못했으면, 화면의 기존 기록을 그대로 둔다.
      const keepShown = quiet && result.root_failed;
      if (!keepShown) data = result.files || {};
      if (!result.root_failed) lastScan = result.scanned_at || 0;
      finishScan();
      if (result.skipped && result.skipped.length > 0) {
        const extra = result.skipped.length > 3 ? ` 외 ${result.skipped.length - 3}건` : "";
        const head = keepShown ? "스캔 폴더를 읽지 못해 이전 기록을 표시합니다: " : "일부 스캔 또는 캐시 저장 실패: ";
        msg.textContent = head + result.skipped.slice(0, 3).join(", ") + extra;
      } else {
        msg.textContent = "";
      }
      if (quiet) {
        // 입력 중이면 입력이 끝난 뒤에 그린다.
        if (isEditing()) document.activeElement.addEventListener("blur", () => setTimeout(renderAll, 0), { once: true });
        else renderAll();
        return;
      }
      animateNext = true;
      renderAll();
      // 스캔 후 자동 전송하지 않음. AI 요약은 사용자가 명시적으로 생성할 때만 동작.
      summaries = await invoke("get_summaries").catch(e => { alert("일별 요약을 불러오지 못했습니다: " + e); return summaries; });
      monthlySummaries = await invoke("get_monthly_summaries").catch(e => { alert("월간 요약을 불러오지 못했습니다: " + e); return monthlySummaries; });
      renderAll();
    });

    await invoke("start_scan");
  } catch (e) {
    finishScan();
    console.error("[startScan] 오류:", e);
    msg.textContent = "스캔을 완료하지 못했습니다: " + (e && e.message ? e.message : e);
  }
}

function key(y, m, d) {
  return y + "-" + String(m + 1).padStart(2, "0") + "-" + String(d).padStart(2, "0");
}

function render() {
  const y = cur.getFullYear(), m = cur.getMonth();
  document.getElementById("yearLabel").textContent = y + "년";
  document.getElementById("label").textContent = (m + 1) + "월";
  const grid = document.getElementById("grid");
  grid.innerHTML = "";
  const first = new Date(y, m, 1).getDay();
  const days = new Date(y, m + 1, 0).getDate();
  const today = new Date();
  let activeDays = 0, totalFiles = 0, busyDate = null, busyCount = 0;
  for (let i = 0; i < first; i++) {
    const pad = document.createElement("div");
    pad.className = "cell";
    grid.appendChild(pad);
  }
  for (let d = 1; d <= days; d++) {
    const cell = document.createElement("div");
    cell.className = "cell";
    if (y === today.getFullYear() && m === today.getMonth() && d === today.getDate())
      cell.classList.add("today");
    const cellKey = key(y, m, d);
    if (cellKey === selectedDate) cell.classList.add("selected");
    if (animateNext) { cell.classList.add("anim"); cell.style.animationDelay = (d * 0.012) + "s"; }
    const num = document.createElement("div");
    num.className = "daynum";
    num.textContent = d;
    cell.appendChild(num);
    const files = filterFiles(data[cellKey] || [], query);
    if (files.length) {
      activeDays++; totalFiles += files.length;
      if (files.length > busyCount) { busyCount = files.length; busyDate = d; }
      cell.classList.add("has-files");
      const listEl = document.createElement("div");
      listEl.className = "file-box";
      listEl.title = "클릭: 파일 목록 열기";
      listEl.onclick = (e) => {
        e.stopPropagation();
        const ae = document.activeElement;
        if (ae && ae.classList.contains("memo-edit")) ae.blur();
        selectDate(cellKey);
      };
      files.slice(0, 2).forEach(f => {
        const row = document.createElement("div");
        row.className = "file-row";
        row.title = f.name;
        const ext = extOf(f.name);
        const badge = document.createElement("span");
        badge.className = "file-badge";
        badge.style.background = chipFor(ext).color;
        badge.textContent = badgeFor(ext);
        const name = document.createElement("span");
        name.className = "file-row-name";
        // 글자 수로 자르지 않는다. 칸 너비에 맞춰 CSS가 말줄임한다.
        name.textContent = f.name.replace(/\.[^.]+$/, "");
        row.appendChild(badge);
        row.appendChild(name);
        row.onclick = (e) => { e.stopPropagation(); showFileMenu(e, f.path, f.name); };
        listEl.appendChild(row);
      });
      if (files.length > 2) {
        const more = document.createElement("div");
        more.className = "file-more";
        more.textContent = `+${files.length - 2} 더보기`;
        more.onclick = (e) => { e.stopPropagation(); selectDate(cellKey); };
        listEl.appendChild(more);
      }
      cell.appendChild(listEl);

      if (summaries[cellKey]) {
        const sumEl = document.createElement("div");
        sumEl.className = "day-summary";
        sumEl.title = "클릭: 타임라인 열기";

        const sumText = document.createElement("span");
        sumText.className = "day-summary-text";
        sumText.textContent = summaries[cellKey];
        sumEl.appendChild(sumText);

        const editBtn = document.createElement("span");
        editBtn.className = "day-summary-edit";
        editBtn.textContent = "✏";
        editBtn.title = "편집";
        sumEl.appendChild(editBtn);

        sumEl.onclick = (e) => { e.stopPropagation(); selectDate(cellKey); };

        const startEdit = (e) => {
          e.stopPropagation();
          const input = document.createElement("input");
          input.className = "day-summary-input";
          input.value = summaries[cellKey] || "";
          sumEl.replaceWith(input);
          input.focus();
          input.select();
          const commit = async () => {
            const val = input.value.trim();
            if (val !== summaries[cellKey]) {
              try {
                await invoke("save_summary", { date: cellKey, text: val });
                if (val) summaries[cellKey] = val; else delete summaries[cellKey];
              } catch (e) { alert("요약을 저장하지 못했습니다: " + e + "\n입력한 내용: " + val); }
            }
            render();
          };
          input.onblur = commit;
          input.onkeydown = (ev) => {
            if (ev.key === "Enter") { ev.preventDefault(); input.blur(); }
            if (ev.key === "Escape") { input.value = summaries[cellKey] || ""; input.blur(); }
          };
        };
        editBtn.onclick = startEdit;
        sumEl.ondblclick = startEdit;

        cell.appendChild(sumEl);
      }
    }
    attachMemo(cell, cellKey);
    grid.appendChild(cell);
  }
  // 마지막 주의 남는 자리도 빈 칸으로 채워 격자가 끊기지 않게 한다.
  for (let i = (7 - ((first + days) % 7)) % 7; i > 0; i--) {
    const pad = document.createElement("div");
    pad.className = "cell";
    grid.appendChild(pad);
  }
  monthStats = { activeDays, totalFiles, busyDate, busyCount };
  animateNext = false;
}

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
    else render();
  };
  ta.onblur = () => finish(true);
  ta.onkeydown = (e) => {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); ta.blur(); }
    else if (e.key === "Escape") { e.preventDefault(); finish(false); }
  };
}

async function saveMemo(cellKey, text) {
  const trimmed = text.trim();
  try {
    await invoke("save_memo", { date: cellKey, text: trimmed });
    if (trimmed) memos[cellKey] = trimmed; else delete memos[cellKey];
  } catch (e) {
    alert("메모를 저장하지 못했습니다: " + e + "\n입력한 내용: " + text);
  }
  render();
}

function renderAll() {
  render();
  renderSidebar();
  renderTimeline();
  renderMonthSummary();
  renderSearch();
}

function monthKey(y, m) {
  return y + "-" + String(m + 1).padStart(2, "0");
}

// 너무 일반적인 토큰(문서 유형·동작어)은 키워드에서 제외
const KW_STOP = new Set([
  "및","관련","최종","수정","수정본","사본","복사본","붙임","별첨","첨부","최종본","초안",
  "버전","자료","파일","문서","결과","보고","보고서","작성","검토","참고","서식","양식",
  "공문","제출","요청","현황","목록","내용","기타","담당","부서","원본","결재","계획서"
]);

// 그 달 파일명에서 자주 나온 키워드 추출(로컬, 외부 전송 없음). [단어, 빈도] 배열 반환.
function monthKeywords(mk, limit = 10) {
  const freq = {};
  for (const dateKey in data) {
    if (!dateKey.startsWith(mk + "-")) continue;
    for (const f of (data[dateKey] || [])) {
      let name = f.name || "";
      const dot = name.lastIndexOf(".");
      if (dot > 0) name = name.slice(0, dot);
      const seen = new Set();
      for (const t of name.split(/[^가-힣A-Za-z]+/)) {
        if (t.length < 2 || KW_STOP.has(t) || seen.has(t)) continue;
        seen.add(t);                       // 파일당 1회만 집계(문서 빈도)
        freq[t] = (freq[t] || 0) + 1;
      }
    }
  }
  let list = Object.entries(freq).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  const strong = list.filter(([, c]) => c >= 2);
  if (strong.length >= 3) list = strong;   // 2회 이상만, 너무 적으면 전체에서
  return list.slice(0, limit);
}

function escapeReg(s) { return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"); }

// 본문 안의 키워드를 인라인 강조 칩으로 감싸 렌더(별도 줄 없이 문장 안에서 강조).
function highlightKeywords(container, text, keywords) {
  container.innerHTML = "";
  const words = (keywords || []).filter(w => w && w.length >= 2);
  if (!words.length) { container.textContent = text; return; }
  const re = new RegExp("(" + words.slice().sort((a, b) => b.length - a.length).map(escapeReg).join("|") + ")", "g");
  let last = 0, m;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) container.appendChild(document.createTextNode(text.slice(last, m.index)));
    const span = document.createElement("span");
    span.className = "kw";
    span.textContent = m[0];
    container.appendChild(span);
    last = m.index + m[0].length;
    if (m.index === re.lastIndex) re.lastIndex++;
  }
  if (last < text.length) container.appendChild(document.createTextNode(text.slice(last)));
}

const IC = {
  cal: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="18" rx="2"/><path d="M16 2v4M8 2v4M3 10h18"/></svg>',
  doc: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 3v4a1 1 0 0 0 1 1h4"/><path d="M17 21H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7l5 5v11a2 2 0 0 1-2 2z"/></svg>',
  fire: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z"/></svg>'
};

function renderMonthSummary() {
  const y = cur.getFullYear(), m = cur.getMonth();
  const title = document.getElementById("monthModalTitle");
  if (title) title.textContent = y + "년 " + (m + 1) + "월";
  const body = document.getElementById("monthSummaryBody");
  if (!body) return;

  const mk = monthKey(y, m);

  // 그 달의 집계(검색 필터와 무관하게 원본 data 기준)
  let activeDays = 0, total = 0, busyDate = null, busyCount = 0;
  const typeCount = {};
  for (const dateKey in data) {
    if (!dateKey.startsWith(mk + "-")) continue;
    const files = data[dateKey] || [];
    if (!files.length) continue;
    activeDays++; total += files.length;
    const day = Number(dateKey.split("-")[2]);
    if (files.length > busyCount) { busyCount = files.length; busyDate = day; }
    for (const f of files) {
      const ext = extOf(f.name);
      const label = badgeFor(ext);
      if (!typeCount[label]) typeCount[label] = { count: 0, color: chipFor(ext).color };
      typeCount[label].count++;
    }
  }

  const statsEl = document.getElementById("mmStats");
  if (statsEl) {
    if (!total) {
      statsEl.innerHTML = '<div class="mm-empty">이 달의 기록이 없습니다.</div>';
    } else {
      const types = Object.entries(typeCount).sort((a, b) => b[1].count - a[1].count);
      const bar = types.map(([l, i]) =>
        `<span class="mm-seg" style="width:${(i.count / total * 100).toFixed(2)}%;background:${i.color}" title="${l} ${i.count}"></span>`).join("");
      const chips = types.map(([l, i]) =>
        `<span class="mm-tchip"><span class="mm-dot" style="background:${i.color}"></span>${l} <b>${i.count}</b></span>`).join("");
      const busyTxt = busyDate ? `${m + 1}월 ${busyDate}일` : "—";
      statsEl.innerHTML =
        `<div class="mm-tiles">
          <div class="mm-tile">
            <div class="mm-ic">${IC.cal}</div>
            <div class="mm-tt"><div class="mm-tv">${activeDays}</div><div class="mm-tl">활동일</div></div>
          </div>
          <div class="mm-tile">
            <div class="mm-ic">${IC.doc}</div>
            <div class="mm-tt"><div class="mm-tv">${total}</div><div class="mm-tl">문서</div></div>
          </div>
          <div class="mm-tile mm-tile-wide mm-tile-busy">
            <div class="mm-tile-l">
              <div class="mm-ic mm-ic-accent">${IC.fire}</div>
              <div class="mm-tt"><div class="mm-tv">${busyCount}<span class="mm-tu">개</span></div><div class="mm-tl">가장 바쁜 날</div></div>
            </div>
            <div class="mm-busy-date">${busyTxt}</div>
          </div>
        </div>
        <div class="mm-bar">${bar}</div>
        <div class="mm-tchips">${chips}</div>`;
    }
  }

  const kwords = monthKeywords(mk, 14).map(e => e[0]);
  const text = monthlySummaries[mk];
  if (text && text.trim()) {
    body.className = "ms-body";
    highlightKeywords(body, text, kwords);
  } else {
    body.className = "ms-body empty";
    body.textContent = total
      ? "아직 이 달의 AI 총평이 없습니다. 아래 ‘AI 월간 총평 생성’을 누르면 만들어집니다."
      : "기록이 없어 생성할 내용이 없습니다.";
  }
}

function renderSidebar() {
  renderMonthNav();
  renderFolders();
  renderScanStatus();
  renderLegend();
  renderStats();
}

function renderMonthNav() {
  const box = document.getElementById("monthNav");
  const y = cur.getFullYear(), m = cur.getMonth();
  box.innerHTML = "";
  const head = document.createElement("div");
  head.className = "mininav-head";
  const prevY = document.createElement("button"); prevY.textContent = "‹";
  prevY.onclick = () => { cur = new Date(y - 1, m, 1); selectedDate = null; animateNext = true; renderAll(); };
  const yLabel = document.createElement("span"); yLabel.className = "y"; yLabel.textContent = y;
  const nextY = document.createElement("button"); nextY.textContent = "›";
  nextY.onclick = () => { cur = new Date(y + 1, m, 1); selectedDate = null; animateNext = true; renderAll(); };
  head.append(prevY, yLabel, nextY);
  box.appendChild(head);

  const grid = document.createElement("div");
  grid.className = "mininav-grid";
  const today = new Date();
  MONTHS.forEach((name, i) => {
    const b = document.createElement("button");
    b.textContent = name;
    if (i === m) b.classList.add("cur");
    b.onclick = () => { cur = new Date(y, i, 1); selectedDate = null; animateNext = true; renderAll(); };
    grid.appendChild(b);
  });
  box.appendChild(grid);

  const todayBtn = document.createElement("button");
  todayBtn.className = "mininav-today";
  todayBtn.textContent = "오늘로";
  todayBtn.onclick = () => { cur = new Date(); selectedDate = null; animateNext = true; renderAll(); };
  box.appendChild(todayBtn);
}

function renderFolders() {
  const box = document.getElementById("sideFolders");
  box.innerHTML = "";
  if (!scanPaths.length) {
    const e = document.createElement("div");
    e.className = "side-empty";
    e.textContent = "폴더 없음 · 관리에서 추가";
    box.appendChild(e);
    return;
  }
  scanPaths.forEach(p => {
    const row = document.createElement("div");
    row.className = "side-folder";
    const s = document.createElement("span");
    // 긴 경로를 왼쪽에서 줄이려고 span이 rtl이다. <bdi>로 감싸 "D:\"의 글자 순서를 지킨다.
    const text = document.createElement("bdi");
    text.textContent = p;
    s.appendChild(text);
    s.title = p;
    row.appendChild(s);
    box.appendChild(row);
  });
}

function formatScanTime(secs) {
  const d = new Date(secs * 1000), now = new Date();
  const hm = String(d.getHours()).padStart(2, "0") + ":" + String(d.getMinutes()).padStart(2, "0");
  const sameDay = d.getFullYear() === now.getFullYear() && d.getMonth() === now.getMonth() && d.getDate() === now.getDate();
  return sameDay ? "오늘 " + hm : (d.getMonth() + 1) + "월 " + d.getDate() + "일 " + hm;
}

function renderScanStatus() {
  const el = document.getElementById("scanStatus");
  if (scanning) el.textContent = "업데이트 중…";
  else el.textContent = lastScan ? "마지막 스캔 " + formatScanTime(lastScan) : "";
}

function renderLegend() {
  const box = document.getElementById("legend");
  box.innerHTML = "";
  LEGEND.forEach(t => {
    const row = document.createElement("div");
    row.className = "legend-row";
    const chip = document.createElement("span");
    chip.className = "type-chip";
    chip.style.background = t.color;
    chip.textContent = t.label;
    const name = document.createElement("span");
    name.textContent = t.name;
    row.append(chip, name);
    box.appendChild(row);
  });
}

function renderStats() {
  const box = document.getElementById("statBox");
  const { activeDays, totalFiles, busyDate, busyCount } = monthStats;
  box.innerHTML = "";
  if (!activeDays) {
    const e = document.createElement("div");
    e.className = "side-empty";
    e.textContent = "이 달의 기록 없음";
    box.appendChild(e);
    return;
  }
  const row = document.createElement("div");
  row.className = "stat-row";
  row.innerHTML = '<span class="stat-num">' + activeDays + '</span><span class="stat-label">일 기록 · '
    + totalFiles + '개 문서</span>';
  box.appendChild(row);
  if (busyDate) {
    const sub = document.createElement("div");
    sub.className = "stat-sub";
    sub.innerHTML = '가장 많은 날 <b>' + (cur.getMonth() + 1) + '월 ' + busyDate + '일</b> · ' + busyCount + '개';
    box.appendChild(sub);
  }
}

function actionBtn(label, handler) {
  const b = document.createElement("button");
  b.textContent = label;
  b.onclick = handler;
  return b;
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
    if (selectedDate && g.date !== selectedDate) {
      const diff = Math.abs(
        (new Date(g.date) - new Date(selectedDate)) / 86400000
      );
      grp.classList.add(diff <= 1 ? "tl-near" : "tl-far");
    }
    const h = document.createElement("div");
    h.className = "tl-date";
    const dateLabel = formatDayLabel(g.date, today);
    const summary = summaries[g.date];
    const datePart = document.createElement("span");
    datePart.textContent = summary ? dateLabel + ", " : dateLabel;
    h.appendChild(datePart);
    if (summary) {
      const sumSpan = document.createElement("span");
      sumSpan.className = "tl-summary";
      sumSpan.textContent = summary;
      sumSpan.title = "클릭: 편집";
      sumSpan.onclick = (e) => {
        e.stopPropagation();
        const gDate = g.date;
        const input = document.createElement("input");
        input.className = "tl-summary-input";
        input.value = summaries[gDate] || "";
        sumSpan.replaceWith(input);
        input.focus(); input.select();
        const commit = async () => {
          const val = input.value.trim();
          if (val !== summaries[gDate]) {
            try {
              await invoke("save_summary", { date: gDate, text: val });
              if (val) summaries[gDate] = val; else delete summaries[gDate];
            } catch (e) { alert("요약을 저장하지 못했습니다: " + e + "\n입력한 내용: " + val); }
          }
          renderTimeline();
        };
        input.onblur = commit;
        input.onkeydown = (ev) => {
          if (ev.key === "Enter") { ev.preventDefault(); input.blur(); }
          if (ev.key === "Escape") { input.value = summaries[gDate] || ""; input.blur(); }
        };
      };
      h.appendChild(sumSpan);
    }
    grp.appendChild(h);
    g.files.forEach(f => grp.appendChild(fileRow(f)));
    tl.appendChild(grp);
  });
}

// 파일 한 줄: 파일명 + (수정 시각 · 폴더) + 열기/폴더/경로 버튼. 드로어와 검색 결과가 같이 쓴다.
function fileRow(f) {
  const row = document.createElement("div");
  row.className = "tl-file";
  const main = document.createElement("span");
  main.className = "tl-main";
  const name = document.createElement("span");
  name.className = "tl-name";
  name.textContent = f.name;
  main.appendChild(name);
  const time = timeOf(f), folder = folderOf(f.path);
  if (time || folder) {
    const meta = document.createElement("span");
    meta.className = "tl-meta";
    if (time) {
      const t = document.createElement("span");
      t.className = "tl-time";
      t.textContent = time;
      meta.appendChild(t);
    }
    if (folder) {
      const fo = document.createElement("span");
      fo.className = "tl-folder";
      fo.title = folder;
      const text = document.createElement("bdi");
      text.textContent = folder;
      fo.appendChild(text);
      meta.appendChild(fo);
    }
    main.appendChild(meta);
  }
  row.appendChild(main);
  const acts = document.createElement("span");
  acts.className = "tl-actions";
  acts.appendChild(actionBtn("열기", e => { e.stopPropagation(); openFile(f.path, "file"); }));
  acts.appendChild(actionBtn("폴더", e => { e.stopPropagation(); openFile(f.path, "folder"); }));
  acts.appendChild(actionBtn("경로", async e => {
    e.stopPropagation();
    const btn = e.currentTarget;
    try { await invoke("copy_text", { text: f.path }); btn.textContent = "복사됨"; }
    catch (_) { btn.textContent = "복사 실패"; }
    setTimeout(() => { btn.textContent = "경로"; }, 1200);
  }));
  row.appendChild(acts);
  row.onclick = () => openFile(f.path, "file");
  return row;
}

function formatFullDate(dateStr) {
  const [y, m, d] = dateStr.split("-").map(Number);
  return y + "년 " + m + "월 " + d + "일 " + WEEKDAYS[new Date(y, m - 1, d).getDay()] + "요일";
}

// 검색어가 있으면 달력 대신 전체 기간의 결과를 최근 날짜부터 보여 준다.
function renderSearch() {
  const box = document.getElementById("searchResults");
  box.innerHTML = "";
  if (!query.trim()) { document.body.removeAttribute("data-search"); return; }
  document.body.setAttribute("data-search", "on");
  const { groups, total, shown } = searchAll(data, query, SEARCH_LIMIT);
  const card = document.createElement("div");
  card.className = "sr-card";
  const head = document.createElement("div");
  head.className = "sr-head";
  head.textContent = !total ? "검색 결과가 없어요."
    : "전체 기간에서 " + total.toLocaleString() + "건"
      + (total > shown ? " · 최근 " + shown.toLocaleString() + "건만 표시합니다. 검색어를 더 넣어 좁혀 보세요." : "");
  card.appendChild(head);
  groups.forEach(g => {
    const grp = document.createElement("div");
    grp.className = "tl-group";
    const h = document.createElement("div");
    h.className = "tl-date sr-date";
    h.title = "이 날짜의 달력으로 이동";
    h.textContent = formatFullDate(g.date);
    const count = document.createElement("span");
    count.className = "sr-count";
    count.textContent = g.count + "건";
    h.appendChild(count);
    h.onclick = () => jumpToDate(g.date);
    grp.appendChild(h);
    g.files.forEach(f => grp.appendChild(fileRow(f)));
    card.appendChild(grp);
  });
  box.appendChild(card);
}

// 검색 결과의 날짜를 누르면 검색을 끝내고 그 달로 가서 그 날을 연다.
function jumpToDate(dateStr) {
  const [y, m] = dateStr.split("-").map(Number);
  cur = new Date(y, m - 1, 1);
  query = "";
  document.getElementById("search").value = "";
  selectedDate = null;
  animateNext = true;
  renderAll();
  selectDate(dateStr);
}

function selectDate(dateStr) {
  selectedDate = (selectedDate === dateStr) ? null : dateStr;
  render();
  renderTimeline();
  if (selectedDate) {
    document.body.setAttribute("data-timeline", "open");
    const grp = document.querySelector('#timeline .tl-group[data-date="' + selectedDate + '"]');
    if (grp) {
      grp.scrollIntoView({ behavior: "smooth", block: "start" });
      grp.classList.add("flash");
      setTimeout(() => grp.classList.remove("flash"), 700);
    }
  } else {
    closeTimeline();
  }
}

function closeTimeline() {
  document.body.removeAttribute("data-timeline");
  if (selectedDate) { selectedDate = null; render(); }
}

async function openFile(path, mode) {
  try {
    await invoke("open_path", { path, mode });
  } catch (e) {
    alert(e);
  }
}

document.getElementById("prev").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() - 1, 1); selectedDate = null; animateNext = true; renderAll(); };
document.getElementById("next").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() + 1, 1); selectedDate = null; animateNext = true; renderAll(); };
window.addEventListener("focus", async () => {
  if (!loaded) { load(); return; }
  if (scanning) return;
  // 보고 있는 달의 파일만 확인한다. 지워진 것이 있을 때만 화면을 다시 그린다.
  const month = monthKey(cur.getFullYear(), cur.getMonth());
  const removed = await invoke("prune_deleted", { month }).catch(() => []);
  if (!removed || !removed.length) return;
  const gone = new Set(removed);
  for (const date of Object.keys(data)) {
    data[date] = data[date].filter(f => !gone.has(f.path));
    if (!data[date].length) delete data[date];
  }
  if (!isEditing()) renderAll();
});
document.getElementById("search").oninput = (e) => { query = e.target.value; renderAll(); };
document.getElementById("search").onkeydown = (e) => {
  if (e.key === "Escape" && query) { e.stopPropagation(); query = ""; e.target.value = ""; renderAll(); }
};
document.getElementById("label").style.cursor = "pointer";
document.getElementById("label").title = "오늘로";
document.getElementById("label").onclick = () => { cur = new Date(); selectedDate = null; animateNext = true; renderAll(); };

let editPaths = [];
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
  const cfg = await invoke("get_config");
  editPaths = (cfg.scan_paths || []).slice();
  editExts = (cfg.extensions || []).slice();
  document.getElementById("apiKeyInput").value = cfg.openai_api_key || "";
  document.getElementById("aiEnabled").checked = !!cfg.ai_summary_enabled;
  renderExts();
  renderPaths();
  document.body.setAttribute("data-settings", "open");
}

function closePanel() {
  document.body.removeAttribute("data-settings");
}

async function addPath() {
  let path = null;
  try { path = await invoke("pick_folder"); } catch (_) {}
  if (path && !editPaths.includes(path)) { editPaths.push(path); renderPaths(); }
}

async function savePaths() {
  if (scanning) {
    document.getElementById("panelMsg").textContent = "스캔이 끝난 후 설정을 저장해 주세요.";
    return;
  }
  const apiKey = document.getElementById("apiKeyInput").value.trim();
  const aiEnabled = document.getElementById("aiEnabled").checked;
  try {
    await invoke("save_config", { scanPaths: editPaths, extensions: editExts, openaiApiKey: apiKey, aiSummaryEnabled: aiEnabled });
  } catch (e) {
    document.getElementById("panelMsg").textContent = e;
    return;
  }
  scanPaths = editPaths.slice();
  closePanel();
  await startScan();
}

// AI 요약 생성 — 명시적 사용자 동작 + 매번 전송 동의 확인.
async function genSummaries() {
  let cfg;
  try { cfg = await invoke("get_config"); } catch (_) { return; }
  if (!cfg.ai_summary_enabled || !(cfg.openai_api_key || "").trim()) {
    alert("AI 요약이 꺼져 있거나 API 키가 없습니다.\n설정에서 'AI 요약 사용'을 켜고 OpenAI API 키를 입력하세요.");
    openPanel();
    return;
  }
  const ok = confirm("최근 90일의 파일명이 OpenAI 서버로 전송됩니다.\n(파일 내용은 전송되지 않습니다.)\n계속할까요?");
  if (!ok) return;
  const btn = document.getElementById("genSummaries");
  const prev = btn ? btn.textContent : "";
  if (btn) { btn.disabled = true; btn.textContent = "생성 중..."; }
  try {
    const s = await invoke("generate_summaries");
    if (s) { summaries = s; renderAll(); }
  } catch (e) {
    alert("요약 생성 실패: " + e);
  } finally {
    if (btn) { btn.disabled = false; btn.textContent = prev; }
  }
}

function openMonthModal() {
  renderMonthSummary();
  document.body.setAttribute("data-month", "open");
}
function closeMonthModal() {
  document.body.removeAttribute("data-month");
}

// AI 월간 총평 생성 — 명시적 사용자 동작 + 매번 전송 동의 확인.
async function genMonthSummary() {
  let cfg;
  try { cfg = await invoke("get_config"); } catch (_) { return; }
  if (!cfg.ai_summary_enabled || !(cfg.openai_api_key || "").trim()) {
    alert("AI 요약이 꺼져 있거나 API 키가 없습니다.\n설정에서 'AI 요약 사용'을 켜고 OpenAI API 키를 입력하세요.");
    openPanel();
    return;
  }
  const y = cur.getFullYear(), m = cur.getMonth();
  const ok = confirm((m + 1) + "월의 파일명이 OpenAI 서버로 전송됩니다.\n(파일 내용은 전송되지 않습니다.)\n계속할까요?");
  if (!ok) return;
  const btn = document.getElementById("genMonthSummary");
  const prev = btn ? btn.textContent : "";
  if (btn) { btn.disabled = true; btn.textContent = "생성 중..."; }
  try {
    const text = await invoke("generate_monthly_summary", { year: y, month: m + 1 });
    monthlySummaries[monthKey(y, m)] = text || "";
    renderMonthSummary();
  } catch (e) {
    alert("월간 총평 생성 실패: " + e);
  } finally {
    if (btn) { btn.disabled = false; btn.textContent = prev; }
  }
}

document.getElementById("fm-open").onclick = () => { if (fileMenuPath) openFile(fileMenuPath, "file"); hideFileMenu(); };
document.getElementById("fm-folder").onclick = () => { if (fileMenuPath) openFile(fileMenuPath, "folder"); hideFileMenu(); };
document.addEventListener("click", hideFileMenu);

document.getElementById("settings").onclick = openPanel;
document.getElementById("manageFolders").onclick = openPanel;
document.getElementById("refreshScan").onclick = () => startScan();
document.getElementById("genSummaries").onclick = genSummaries;
document.getElementById("genMonthSummary").onclick = genMonthSummary;
document.getElementById("openMonthModal").onclick = openMonthModal;
document.getElementById("monthModalClose").onclick = closeMonthModal;
document.getElementById("monthModal").onclick = (e) => { if (e.target.id === "monthModal") closeMonthModal(); };
document.getElementById("addPath").onclick = addPath;
document.getElementById("addExt").onclick = addExt;
document.getElementById("savePaths").onclick = savePaths;
document.getElementById("closePanel").onclick = closePanel;
document.getElementById("scrim").onclick = () => { closePanel(); closeTimeline(); };
document.addEventListener("keydown", (e) => { if (e.key === "Escape") { closePanel(); closeTimeline(); closeMonthModal(); } });
load();
