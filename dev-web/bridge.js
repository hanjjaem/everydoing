/* Development-only adapter: real local backend, no mock data. */
(() => {
  const handlers = new Map();
  const token = document.querySelector('meta[name="everydoing-session"]').content;
  async function invoke(command, args = {}) {
    if (command === 'pick_folder') {
      return window.prompt('스캔할 폴더의 전체 경로를 입력하세요.');
    }
    const response = await fetch('/api/invoke', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'X-Everydoing-Session': token },
      body: JSON.stringify({ command, args })
    });
    const payload = await response.json();
    if (!response.ok) throw new Error(payload.error || '요청 처리에 실패했습니다.');
    if (command === 'start_scan') {
      for (const callback of handlers.get('scan-done') || []) callback({ payload: payload.result });
      return null;
    }
    return payload.result;
  }
  async function listen(name, callback) {
    if (!handlers.has(name)) handlers.set(name, new Set());
    handlers.get(name).add(callback);
    return () => handlers.get(name).delete(callback);
  }
  window.__TAURI__ = { core: { invoke }, event: { listen } };
  document.addEventListener('DOMContentLoaded', () => {
    const note = document.createElement('p');
    note.textContent = '브라우저 개발 모드: 메모는 별도 저장되며 AI 요약은 데스크톱 앱에서 사용합니다.';
    note.style.cssText = 'margin:8px 0;color:#515b6b;font-size:12px;line-height:1.4';
    document.querySelector('#statBox')?.before(note);
    for (const id of ['genSummaries', 'genMonthSummary']) {
      const button = document.getElementById(id);
      if (button) { button.disabled = true; button.title = '브라우저 개발 모드에서 AI 요약을 지원하지 않습니다.'; }
    }
  });
})();
