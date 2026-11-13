const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

// app.js에서 화면(DOM)과 무관한 계산 함수 구간만 떼어 실행한다(search.test.cjs와 같은 구간).
const source = fs.readFileSync('static/app.js', 'utf8');
const helpers = source.slice(source.indexOf('function matchesQuery'), source.indexOf('const WEEKDAYS'));
const app = {};
vm.runInNewContext(helpers, app);

const secs = (y, mo, d, h, mi) => new Date(y, mo - 1, d, h, mi).getTime() / 1000;
const now = new Date(2026, 10, 13, 15, 0);
const status = (fields) => ({
  current_version: '0.3.0', latest_version: null, checked_at: 0,
  update_available: false, check_enabled: false, ...fields,
});

test('확인 기록이 없으면 아직 확인하지 않았다고 쓴다', () => {
  assert.equal(app.updateStatusText(status({}), now), '현재 0.3.0 · 아직 확인하지 않았습니다');
});

test('최신이면 최신 버전이라고 쓰고, 오늘 확인했으면 "오늘 HH:MM"', () => {
  const s = status({ latest_version: '0.3.0', checked_at: secs(2026, 11, 13, 9, 5) });
  assert.equal(app.updateStatusText(s, now), '현재 0.3.0 · 최신 버전입니다 · 마지막 확인 오늘 09:05');
});

test('새 버전이 있으면 그 번호를 쓰고, 다른 날 확인했으면 "M월 D일 HH:MM"', () => {
  const s = status({ latest_version: '0.3.1', checked_at: secs(2026, 11, 12, 9, 0), update_available: true });
  assert.equal(app.updateStatusText(s, now), '현재 0.3.0 · 새 버전이 있습니다(0.3.1) · 마지막 확인 11월 12일 09:00');
});

test('상태를 모르면 빈 문자열', () => {
  assert.equal(app.updateStatusText(null, now), '');
});

test('사이드바 버전 배지는 확인 중·오류·최신·업데이트 가능 상태를 구분한다', () => {
  const checking = app.formatSidebarUpdate(status({ check_enabled: true }), { checking: true }, now);
  assert.equal(checking.state, 'checking');
  assert.equal(checking.text, 'v0.3.0 · 새 버전 확인 중…');

  const failed = app.formatSidebarUpdate(status({}), { error: 'timeout' }, now);
  assert.equal(failed.state, 'error');
  assert.equal(failed.text, 'v0.3.0 · 확인 실패 (다시 확인)');
  assert.equal(failed.title, 'timeout');

  const latest = app.formatSidebarUpdate(status({ latest_version: '0.3.0', checked_at: secs(2026, 11, 13, 9, 5) }), {}, now);
  assert.equal(latest.state, 'ok');
  assert.equal(latest.text, 'v0.3.0 · 최신 (오늘 09:05 확인)');

  const avail = app.formatSidebarUpdate(status({ latest_version: '0.3.1', checked_at: secs(2026, 11, 13, 9, 5), update_available: true }), {}, now);
  assert.equal(avail.state, 'available');
  assert.equal(avail.text, 'v0.3.0 → 0.3.1 업데이트 가능');

  const preview = app.formatSidebarUpdate(status({}), { desktop: false }, now);
  assert.equal(preview.text, 'v0.3.0 · 개발 미리보기');
});
