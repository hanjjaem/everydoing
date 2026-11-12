const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');

const source = fs.readFileSync('static/app.js', 'utf8').split('\nlet data =')[0];

function createPage(nativeWindow) {
  const events = {};
  const elements = Object.fromEntries(['window-minimize', 'window-maximize', 'window-close'].map(id => {
    const classes = new Set();
    return [id, {
      events: {}, attributes: {}, title: '',
      classList: { toggle: (name, enabled) => enabled ? classes.add(name) : classes.delete(name), contains: name => classes.has(name) },
      addEventListener(name, handler) { this.events[name] = handler; },
      setAttribute(name, value) { this.attributes[name] = value; },
    }];
  }));
  const rootClasses = new Set();
  const window = {
    __TAURI__: { core: { invoke() {} }, event: { listen() {} }, ...(nativeWindow ? { window: { getCurrentWindow: () => nativeWindow } } : {}) },
    addEventListener(name, handler) { events[name] = handler; },
  };
  const document = {
    documentElement: { classList: { add: name => rootClasses.add(name) } },
    getElementById: id => elements[id],
  };
  vm.runInNewContext(source, { window, document, console });
  return { elements, events, rootClasses };
}

test('desktop controls call the window API and reflect maximize state', async () => {
  const calls = [];
  let maximized = false;
  const page = createPage({
    isMaximized: async () => maximized,
    minimize: async () => calls.push('minimize'),
    toggleMaximize: async () => { maximized = !maximized; calls.push('maximize'); },
    close: async () => calls.push('close'),
  });
  assert.equal(page.rootClasses.has('desktop-shell'), true);
  await page.elements['window-maximize'].events.click();
  assert.equal(page.elements['window-maximize'].attributes['aria-label'], '이전 크기로');
  assert.equal(page.elements['window-maximize'].classList.contains('is-maximized'), true);
  await page.elements['window-maximize'].events.click();
  assert.equal(page.elements['window-maximize'].attributes['aria-label'], '최대화');
  page.elements['window-minimize'].events.click();
  page.elements['window-close'].events.click();
  assert.deepEqual(calls, ['maximize', 'maximize', 'minimize', 'close']);
  maximized = true;
  await page.events.resize();
  assert.equal(page.elements['window-maximize'].attributes['aria-label'], '이전 크기로');
});

test('browser development mode does not enable native window controls', () => {
  const page = createPage();
  assert.equal(page.rootClasses.has('desktop-shell'), false);
  assert.deepEqual(Object.keys(page.elements['window-close'].events), []);
});

test('desktop window enables dragging and permits its controls', () => {
  const config = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'));
  const capabilities = JSON.parse(fs.readFileSync('src-tauri/capabilities/default.json', 'utf8'));
  const html = fs.readFileSync('static/index.html', 'utf8');
  assert.equal(config.app.windows[0].decorations, false);
  assert.match(html, /class="window-drag-region" data-tauri-drag-region/);
  for (const action of ['start-dragging', 'minimize', 'toggle-maximize', 'close']) {
    assert.ok(capabilities.permissions.includes(`core:window:allow-${action}`));
  }
});
