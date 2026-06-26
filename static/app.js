const invoke = window.__TAURI__.core.invoke;

let data = {};
let cur = new Date();
let query = "";
let selectedDate = null;
let memos = {};
let animateNext = true;
let scanPaths = [];
let monthStats = { activeDays: 0, totalFiles: 0, busyDate: null, busyCount: 0 };

const MONTHS = ["1월","2월","3월","4월","5월","6월","7월","8월","9월","10월","11월","12월"];
const LEGEND = [
  { color: "#1E6FE8", label: "한", name: "한글 (hwp)" },
  { color: "#2B579A", label: "W", name: "워드 (doc)" },
  { color: "#217346", label: "X", name: "엑셀 (xls)" },
  { color: "#C43E1C", label: "P", name: "PPT (ppt)" },
  { color: "#E5252A", label: "F", name: "PDF" },
];

function filterFiles(files, q) {
  const norm = (q || "").trim().toLowerCase();
  if (!norm) return files;
  return files.filter(f => (f.name || "").toLowerCase().includes(norm));
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
  try { memos = await invoke("get_memos"); } catch (_) { memos = {}; }
  try { scanPaths = (await invoke("get_config")).scan_paths || []; } catch (_) { scanPaths = []; }

  if (scanPaths.length === 0) {
    msg.textContent = "";
    data = {};
    renderAll();
    openPanel();
    return;
  }

  msg.textContent = "인덱싱 중...";
  animateNext = true;
  try {
    const result = await invoke("list_files");
    data = result.files;
    if (result.skipped && result.skipped.length > 0) {
      msg.textContent = "건너뛴 드라이브: " + result.skipped.join(", ") + " (NTFS가 아니거나 읽을 수 없음)";
    } else {
      msg.textContent = "";
    }
  } catch (e) {
    msg.textContent = "파일 스캔 실패: " + e;
    data = {};
  }
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
      const act = document.createElement("div");
      act.className = "act";
      act.onclick = (e) => {
        e.stopPropagation();
        const ae = document.activeElement;
        if (ae && ae.classList.contains("memo-edit")) ae.blur();
        selectDate(cellKey);
      };
      const { chips, plus } = typeChips(files);
      const chipsEl = document.createElement("span");
      chipsEl.className = "act-chips";
      chips.forEach(c => {
        const chip = document.createElement("span");
        chip.className = "type-chip";
        chip.style.background = c.color;
        chip.textContent = c.label;
        chipsEl.appendChild(chip);
      });
      if (plus) {
        const more = document.createElement("span");
        more.className = "more";
        more.textContent = "+";
        chipsEl.appendChild(more);
      }
      act.appendChild(chipsEl);

      const countEl = document.createElement("span");
      countEl.className = "act-count";
      countEl.textContent = files.length;
      act.appendChild(countEl);
      cell.appendChild(act);
    }
    attachMemo(cell, cellKey);
    grid.appendChild(cell);
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
  if (trimmed) memos[cellKey] = trimmed; else delete memos[cellKey];
  render();
  try { await invoke("save_memo", { date: cellKey, text: trimmed }); } catch (_) {}
}

function renderAll() {
  render();
  renderSidebar();
  renderTimeline();
}

function renderSidebar() {
  renderMonthNav();
  renderFolders();
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
    s.textContent = p;
    s.title = p;
    row.appendChild(s);
    box.appendChild(row);
  });
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
      acts.appendChild(actionBtn("경로", e => { e.stopPropagation(); invoke("copy_text", { text: f.path }); }));
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
    await invoke("open_path", { path, mode });
  } catch (e) {
    alert(e);
  }
}

document.getElementById("prev").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() - 1, 1); selectedDate = null; animateNext = true; renderAll(); };
document.getElementById("next").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() + 1, 1); selectedDate = null; animateNext = true; renderAll(); };
window.addEventListener("focus", load);
document.getElementById("search").oninput = (e) => { query = e.target.value; renderAll(); };
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
  try {
    await invoke("save_config", { scanPaths: editPaths, extensions: editExts });
  } catch (e) {
    document.getElementById("panelMsg").textContent = e;
    return;
  }
  closePanel();
  load();
}

document.getElementById("settings").onclick = openPanel;
document.getElementById("manageFolders").onclick = openPanel;
document.getElementById("addPath").onclick = addPath;
document.getElementById("addExt").onclick = addExt;
document.getElementById("savePaths").onclick = savePaths;
document.getElementById("closePanel").onclick = closePanel;
document.getElementById("scrim").onclick = () => { closePanel(); closeTimeline(); };
document.addEventListener("keydown", (e) => { if (e.key === "Escape") { closePanel(); closeTimeline(); } });
load();
