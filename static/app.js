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

let data = {};
let cur = new Date();
let query = "";
let selectedDate = null;
let memos = {};

function filterFiles(files, q) {
  const norm = (q || "").trim().toLowerCase();
  if (!norm) return files;
  return files.filter(f => (f.name || "").toLowerCase().includes(norm));
}

const WEEKDAYS = ["일", "월", "화", "수", "목", "금", "토"];

function amtLevel(n) {
  if (n <= 2) return 1;
  if (n <= 7) return 2;
  return 3;
}

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

async function load() {
  const msg = document.getElementById("msg");
  msg.textContent = "";
  try {
    const res = await fetch("/api/files");
    if (!res.ok) {
      let detail = "";
      try { detail = " (" + (await res.json()).error + ")"; } catch (_) {}
      msg.textContent = "Everything HTTP 서버에 연결할 수 없습니다. " +
        "Everything → 도구 → 옵션 → HTTP 서버를 켰는지, 포트가 config.json과 맞는지 확인하세요." + detail;
      data = {};
    } else {
      data = await res.json();
    }
  } catch (e) {
    msg.textContent = "서버 응답을 받을 수 없습니다: " + e;
    data = {};
  }
  try {
    const r = await fetch("/api/memos");
    memos = r.ok ? await r.json() : {};
  } catch (_) { memos = {}; }
  renderAll();
}

function key(y, m, d) {
  return y + "-" + String(m + 1).padStart(2, "0") + "-" + String(d).padStart(2, "0");
}

function render() {
  const y = cur.getFullYear(), m = cur.getMonth();
  document.getElementById("yearLabel").textContent = y;
  document.getElementById("label").textContent = (m + 1) + "월";
  const grid = document.getElementById("grid");
  grid.innerHTML = "";
  const first = new Date(y, m, 1).getDay();
  const days = new Date(y, m + 1, 0).getDate();
  const today = new Date();
  let activeDays = 0, totalFiles = 0;
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
    const num = document.createElement("div");
    num.className = "daynum";
    num.textContent = d;
    cell.appendChild(num);
    const files = filterFiles(data[cellKey] || [], query);
    if (files.length) {
      activeDays++; totalFiles += files.length;
      cell.classList.add("has-files");
      const count = document.createElement("span");
      count.className = "count";
      count.dataset.amt = amtLevel(files.length);
      count.textContent = files.length;
      count.onclick = (e) => {
        e.stopPropagation();
        const ae = document.activeElement;
        if (ae && ae.classList.contains("memo-edit")) ae.blur();
        selectDate(cellKey);
      };
      cell.appendChild(count);

      const { chips, plus } = typeChips(files);
      const typeWrap = document.createElement("span");
      typeWrap.className = "types";
      chips.forEach(c => {
        const chip = document.createElement("span");
        chip.className = "type-chip";
        chip.style.background = c.color;
        chip.textContent = c.label;
        typeWrap.appendChild(chip);
      });
      if (plus) {
        const more = document.createElement("span");
        more.className = "more";
        more.textContent = "+";
        typeWrap.appendChild(more);
      }
      cell.appendChild(typeWrap);
    }
    attachMemo(cell, cellKey);
    grid.appendChild(cell);
  }
  const summary = document.getElementById("summary");
  summary.innerHTML = activeDays
    ? '<b>' + activeDays + '</b>일 기록 · <b>' + totalFiles + '</b>개 문서'
    : '이 달의 기록 없음';
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

function renderAll() {
  render();
  renderTimeline();
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

function selectDate(dateStr) {
  selectedDate = (selectedDate === dateStr) ? null : dateStr;
  render();
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
    const res = await fetch("/api/open", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path, mode }),
    });
    if (!res.ok) alert("파일을 찾을 수 없습니다");
  } catch (e) {
    alert("서버에 연결할 수 없습니다: " + e);
  }
}

document.getElementById("prev").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() - 1, 1); selectedDate = null; renderAll(); };
document.getElementById("next").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() + 1, 1); selectedDate = null; renderAll(); };
window.addEventListener("focus", load);
document.getElementById("search").oninput = (e) => { query = e.target.value; renderAll(); };
document.getElementById("label").style.cursor = "pointer";
document.getElementById("label").title = "오늘로";
document.getElementById("label").onclick = () => { cur = new Date(); selectedDate = null; renderAll(); };
document.getElementById("themeToggle").onclick = toggleTheme;
applyTheme(document.documentElement.getAttribute("data-theme") === "dark" ? "dark" : "light");

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
  const res = await fetch("/api/config");
  const cfg = await res.json();
  editPaths = (cfg.scan_paths || []).slice();
  editExts = (cfg.extensions || []).slice();
  renderExts();
  renderPaths();
  document.body.setAttribute("data-settings", "open");
}

function closePanel() {
  document.body.removeAttribute("data-settings");
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
    body: JSON.stringify({ scan_paths: editPaths, extensions: editExts }),
  });
  if (!res.ok) {
    let msg = "저장 실패";
    try { msg = (await res.json()).error || msg; } catch (_) {}
    document.getElementById("panelMsg").textContent = msg;
    return;
  }
  closePanel();
  load();
}

document.getElementById("settings").onclick = openPanel;
document.getElementById("addPath").onclick = addPath;
document.getElementById("addExt").onclick = addExt;
document.getElementById("savePaths").onclick = savePaths;
document.getElementById("closePanel").onclick = closePanel;
document.getElementById("scrim").onclick = () => { closePanel(); closeTimeline(); };
document.addEventListener("keydown", (e) => { if (e.key === "Escape") { closePanel(); closeTimeline(); } });
load();
