const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

// app.js에서 화면(DOM)과 무관한 계산 함수 구간만 떼어 실행한다.
const source = fs.readFileSync('static/app.js', 'utf8');
const helpers = source.slice(source.indexOf('function matchesQuery'), source.indexOf('const WEEKDAYS'));
const app = {};
vm.runInNewContext(helpers, app);

test('진행 이벤트가 없으면 준비 중과 경과 초를 만든다', () => {
  const init = app.formatScanProgress(null, 0);
  assert.equal(init.pct, null);
  assert.equal(init.headline, '스캔 준비 중...');
  assert.equal(init.detail, '');
  assert.equal(init.sidebarText, '스캔 준비 중');
  assert.equal(app.formatScanProgress(null, 3).sidebarText, '스캔 준비 중 · 3초');
});

test('전체 레코드 수가 있으면 0~100 퍼센트와 개수를 계산한다', () => {
  const out = app.formatScanProgress({ drive: 'D:\\', scanned: 320000, total: 500000, matched: 1240 }, 4);
  assert.equal(out.pct, 64);
  assert.equal(out.headline, 'D:\\ 스캔 중 · 64%');
  assert.equal(out.detail, '320,000 / 500,000 · 파일 1,240개 발견');
  assert.equal(out.sidebarText, '스캔 중 · D:\\ 64% · 1,240개');
});

test('레코드 수가 total을 넘어도 100%에서 멈춘다', () => {
  const out = app.formatScanProgress({ drive: 'D:\\', scanned: 520000, total: 500000, matched: 10 }, 5);
  assert.equal(out.pct, 100);
});

test('total이 0인 폴더 순회에서는 확인한 항목 수와 발견 파일 수를 보여 준다', () => {
  const out = app.formatScanProgress({ drive: 'C:\\scan', scanned: 1400, total: 0, matched: 38 }, 2);
  assert.equal(out.pct, null);
  assert.equal(out.headline, 'C:\\scan 스캔 중');
  assert.equal(out.detail, '1,400개 확인 · 파일 38개 발견');
  assert.equal(out.sidebarText, '스캔 중 · C:\\scan · 1,400개 확인 (38개)');
});

test('이전 색인과 새 스캔 결과를 비교해 새 문서와 수정·삭제 건수를 구분한다', () => {
  const prev = {
    '2026-11-12': [
      { name: 'a.pdf', path: 'D:\\a.pdf', modified: 100 },
      { name: 'b.hwp', path: 'D:\\b.hwp', modified: 200 },
      { name: 'gone.doc', path: 'D:\\gone.doc', modified: 50 },
    ],
  };
  const next = {
    '2026-11-12': [
      { name: 'a.pdf', path: 'D:\\a.pdf', modified: 100 },
      { name: 'b.hwp', path: 'D:\\b.hwp', modified: 250 },
    ],
    '2026-11-13': [
      { name: 'new.xlsx', path: 'D:\\new.xlsx', modified: 300 },
    ],
  };
  const diff = app.diffScanResults(prev, next);
  assert.equal(diff.hadPrev, true);
  assert.equal(diff.added, 1);
  assert.equal(diff.updated, 1);
  assert.equal(diff.removed, 1);
  assert.equal(diff.total, 3);
  assert.equal(diff.freshPaths.has('D:\\new.xlsx'), true);
  assert.equal(diff.freshPaths.has('D:\\b.hwp'), true);
  assert.equal(diff.freshPaths.has('D:\\a.pdf'), false);
  assert.equal(app.formatScanDiff(diff), '새 문서 1개 · 수정 1개 · 정리 1개 반영');

  const same = app.diffScanResults(next, next);
  assert.equal(app.formatScanDiff(same), '새 문서·변동 없음 (총 3개)');
});
