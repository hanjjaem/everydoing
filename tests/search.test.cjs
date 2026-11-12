const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

// app.js에서 화면(DOM)과 무관한 계산 함수 구간만 떼어 실행한다.
const source = fs.readFileSync('static/app.js', 'utf8');
const helpers = source.slice(source.indexOf('function matchesQuery'), source.indexOf('const WEEKDAYS'));
const app = {};
vm.runInNewContext(helpers, app);

const file = (name, folder, modified = 0) => ({ name, path: folder + '\\' + name, modified });
const data = {
  '2026-11-10': [file('예산 집계표.xlsx', 'D:\\업무\\예산'), file('회의록.hwp', 'D:\\업무\\공모전')],
  '2026-03-02': [file('공모전 계획.hwp', 'D:\\업무\\기획')],
  '2025-12-24': [file('예산 요구서.hwp', 'D:\\업무\\예산')],
  unknown: [file('예산 메모.hwp', 'D:\\업무')],
};

test('검색은 보고 있는 달과 상관없이 전체 기간에서 최근 날짜부터 찾는다', () => {
  const { groups, total, shown } = app.searchAll(data, '예산', 300);
  // vm 안에서 만든 배열은 원형이 달라서, 바깥 배열로 옮겨 비교한다.
  assert.deepEqual(Array.from(groups, g => g.date), ['2026-11-10', '2025-12-24']);
  assert.equal(total, 2);
  assert.equal(shown, 2);
});

test('파일명에 없어도 폴더 이름으로 찾는다', () => {
  const { groups } = app.searchAll(data, '공모전', 300);
  assert.deepEqual(Array.from(groups, g => Array.from(g.files, f => f.name)), [['회의록.hwp'], ['공모전 계획.hwp']]);
});

test('대소문자를 가리지 않고, 빈 검색어는 결과가 없다', () => {
  assert.equal(app.searchAll({ '2026-01-05': [file('Report.PDF', 'D:\\w')] }, 'report.pdf', 300).total, 1);
  assert.equal(app.searchAll(data, '   ', 300).total, 0);
});

test('표시 개수는 제한하되 전체 건수는 끝까지 센다', () => {
  const many = { '2026-11-10': [1, 2, 3].map(n => file(`보고 ${n}.hwp`, 'D:\\w')), '2026-11-09': [file('보고 4.hwp', 'D:\\w')] };
  const { groups, total, shown } = app.searchAll(many, '보고', 2);
  assert.equal(total, 4);
  assert.equal(shown, 2);
  assert.equal(groups.length, 1);
  assert.equal(groups[0].count, 3, '그 날짜의 전체 건수');
  assert.equal(groups[0].files.length, 2);
});

test('수정 시각은 그 PC의 현지 시각 HH:MM으로, 모르면 빈 문자열로 보여 준다', () => {
  const secs = new Date(2026, 10, 12, 9, 5).getTime() / 1000;
  assert.equal(app.timeOf({ modified: secs }), '09:05');
  assert.equal(app.timeOf({ modified: 0 }), '');
  assert.equal(app.timeOf({}), '');
});

test('월 인덱스의 점: 그 해에 기록이 있는 달만 고른다', () => {
  const months = app.monthsWithRecords({ ...data, '2026-08-03': [] }, 2026);
  assert.deepEqual(Array.from(months).sort((a, b) => a - b), [2, 10], '3월과 11월. 파일이 없는 날만 있는 8월은 빠진다');
  assert.deepEqual(Array.from(app.monthsWithRecords(data, 2025)), [11]);
  assert.equal(app.monthsWithRecords(data, 2027).size, 0);
});

test('폴더 경로는 파일명을 뗀 앞부분이다', () => {
  assert.equal(app.folderOf('D:\\업무\\예산\\집계표.xlsx'), 'D:\\업무\\예산');
  assert.equal(app.folderOf('집계표.xlsx'), '');
});
