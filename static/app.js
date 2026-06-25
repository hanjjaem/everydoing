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

function filterFiles(files, q) {
  const norm = (q || "").trim().toLowerCase();
  if (!norm) return files;
  return files.filter(f => (f.name || "").toLowerCase().includes(norm));
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
  render();
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
    const num = document.createElement("div");
    num.className = "daynum";
    num.textContent = d;
    cell.appendChild(num);
    const files = filterFiles(data[key(y, m, d)] || [], query);
    if (files.length) {
      activeDays++; totalFiles += files.length;
      cell.classList.add("has-files");
      const badge = document.createElement("span");
      badge.className = "badge";
      badge.innerHTML = '<span class="n"></span><span class="u">개</span>';
      badge.querySelector(".n").textContent = files.length;
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
  const summary = document.getElementById("summary");
  summary.innerHTML = activeDays
    ? '<b>' + activeDays + '</b>일 기록 · <b>' + totalFiles + '</b>개 문서'
    : '이 달의 기록 없음';
}

function menu(f) {
  const choice = prompt(f.name + "\n1: 파일 열기  2: 폴더 열기  3: 경로 복사\n번호 입력:", "1");
  if (choice === "1") openFile(f.path, "file");
  else if (choice === "2") openFile(f.path, "folder");
  else if (choice === "3") navigator.clipboard.writeText(f.path);
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

document.getElementById("prev").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() - 1, 1); render(); };
document.getElementById("next").onclick = () => { cur = new Date(cur.getFullYear(), cur.getMonth() + 1, 1); render(); };
document.getElementById("refresh").onclick = load;
document.getElementById("search").oninput = (e) => { query = e.target.value; render(); };
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
    body: JSON.stringify({ scan_paths: editPaths, extensions: editExts }),
  });
  if (!res.ok) {
    let msg = "저장 실패";
    try { msg = (await res.json()).error || msg; } catch (_) {}
    document.getElementById("panelMsg").textContent = msg;
    return;
  }
  document.getElementById("panel").style.display = "none";
  load();
}

document.getElementById("settings").onclick = openPanel;
document.getElementById("addPath").onclick = addPath;
document.getElementById("addExt").onclick = addExt;
document.getElementById("savePaths").onclick = savePaths;
document.getElementById("closePanel").onclick = () => { document.getElementById("panel").style.display = "none"; };
load();
