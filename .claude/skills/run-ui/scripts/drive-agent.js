#!/usr/bin/env node
// Headless smoke-driver for a Newbound instance's web UI (agent chat).
// Verified end-to-end 2026-08-25; the gotchas it encodes are documented
// in ../SKILL.md — read that before changing the flow.
//
// Usage:
//   BASE_URL=http://127.0.0.1:8080 NB_USER=admin NB_PASS=... \
//   PROMPT="Reply with exactly: QA OK" SHOT_DIR=./shots \
//   [PW_CHROMIUM=/opt/pw-browsers/chromium] [REPLY_TIMEOUT_MS=240000] \
//   node drive-agent.js
//
// Pass: exit 0, screenshots in SHOT_DIR, thread text between
// THREAD_TEXT_START/END on stdout. Fail: exit 1 with DRIVER_FAIL.

// The script lives outside the QA workspace, so resolve playwright from
// the invoking directory too (the documented usage: run from a dir that
// has node_modules/playwright).
let chromium;
try {
  ({ chromium } = require('playwright'));
} catch {
  const { createRequire } = require('module');
  ({ chromium } = createRequire(process.cwd() + '/')('playwright'));
}

const BASE = process.env.BASE_URL || 'http://127.0.0.1:8080';
const USER = process.env.NB_USER || 'admin';
const PASS = process.env.NB_PASS;
const PROMPT = process.env.PROMPT || 'Reply with exactly: QA OK';
const SHOTS = process.env.SHOT_DIR || '.';
const REPLY_TIMEOUT_MS = parseInt(process.env.REPLY_TIMEOUT_MS || '240000', 10);

if (!PASS) { console.error('DRIVER_FAIL NB_PASS is required (see users/admin.properties)'); process.exit(1); }

(async () => {
  const launchOpts = { args: ['--no-sandbox'] };
  if (process.env.PW_CHROMIUM) launchOpts.executablePath = process.env.PW_CHROMIUM;
  const browser = await chromium.launch(launchOpts);
  const page = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
  page.on('console', (m) => { if (m.type() === 'error') console.log('[console.error]', m.text()); });

  // Login lives on the home page, NOT on app pages: an unauthenticated
  // app page serves an empty shell and never shows a form.
  await page.goto(BASE + '/', { waitUntil: 'domcontentloaded' });
  const login = await page.waitForSelector('#password', { timeout: 45000 }).catch(() => null);
  if (login) {
    await page.fill('#username', USER);
    await page.fill('#password', PASS);
    await page.screenshot({ path: SHOTS + '/1-login.png' });
    await page.click('.loginbutton');
    await page.waitForFunction(() => !document.querySelector('#password'), null, { timeout: 20000 })
      .catch(() => {});
    await page.waitForTimeout(1500);
  }

  await page.goto(BASE + '/agent/index.html', { waitUntil: 'domcontentloaded' });
  await page.waitForSelector('.ag-input', { timeout: 45000 });
  await page.screenshot({ path: SHOTS + '/2-chat.png' });

  await page.fill('.ag-input', PROMPT);
  const before = await page.locator('.ag-thread > *').count();
  await page.click('.ag-send');

  // The thread appends spinner/streaming cells before the final one:
  // wait for a new, non-empty last cell whose text is STABLE across
  // two consecutive polls, not for a child count.
  let prev = '';
  let stable = 0;
  const deadline = Date.now() + REPLY_TIMEOUT_MS;
  while (Date.now() < deadline) {
    await page.waitForTimeout(2000);
    const state = await page.evaluate((n) => {
      const t = document.querySelector('.ag-thread');
      if (!t || t.children.length <= n) return null;
      const last = t.lastElementChild;
      return last ? (last.textContent || '').trim() : null;
    }, before);
    if (state && !/^(thinking|…|\.\.\.)/i.test(state)) {
      if (state === prev) { stable += 1; if (stable >= 2) break; }
      else { stable = 0; prev = state; }
    }
  }
  if (stable < 2) throw new Error(`no stable agent reply within ${REPLY_TIMEOUT_MS}ms`);

  await page.screenshot({ path: SHOTS + '/3-answer.png' });
  console.log('THREAD_TEXT_START');
  console.log((await page.locator('.ag-thread').innerText()).slice(0, 2500));
  console.log('THREAD_TEXT_END');
  await browser.close();
})().catch((e) => { console.error('DRIVER_FAIL', e.message || e); process.exit(1); });
