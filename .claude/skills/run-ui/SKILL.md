---
name: run-ui
description: Launch a local Newbound instance and drive its web UI headlessly with Playwright — deterministic admin login, agent-chat smoke test, screenshots. Use when asked to run the app, verify a UI change in the real browser, or do UI/QA work against the platform or the agent app.
---

# Driving the Newbound UI headlessly

Verified cold-start from a headless Linux container (2026-08-25): server
up, login, agent chat round-trip through a live LLM arm, screenshots.
Every gotcha below was actually hit; don't rediscover them.

## 1. Prerequisites

- A built platform binary: `<newbound>/target/release/newbound`
  (`tools/setup.sh` in this repo builds it and stages the overlay).
- The app under test listed in `<newbound>/config.properties` `apps=`
  (e.g. `apps=app,dev,security,peer,agent`).
- Playwright: `npm install playwright` somewhere writable. If the
  container pre-installs browsers (`PLAYWRIGHT_BROWSERS_PATH` set,
  e.g. `/opt/pw-browsers`) and the npm package wants a different
  browser build, do NOT run `playwright install` — launch with
  `executablePath` pointing at the preinstalled binary instead
  (`/opt/pw-browsers/chromium`). Root containers need `--no-sandbox`.

## 2. Deterministic login (do this BEFORE first server start)

On first run with `security=on`, the server generates
`<newbound>/users/admin.properties` with a RANDOM password. Pre-create
it instead so QA logins are deterministic:

```properties
# <newbound>/users/admin.properties
displayname=System Administrator
groups=admin
password=<your-test-password>
```

Users load once at startup (`security.init` → `load_users`), so create
this before launching, or restart after changing it. Never commit it.

## 3. Optional: a live LLM behind the chat

Chat QA needs `runtime/agent/botd.properties` (live keys, re-read per
call, no restart). The cheapest real arm in a Claude Code container is
the CLAUDECODE bridge (see `docs/claudecode-arm.md`):

```properties
LLM=CLAUDECODE
LLM_CTL=agent:llm:claude_code
CLAUDE_CODE_MODEL=claude-sonnet-5
```

Pin a small model: an unpinned child inherits the session default and
a single chat turn can cost ~200x more. Expect 10–60s per reply —
size driver timeouts accordingly.

## 4. Server lifecycle

```bash
cd <newbound> && ./target/release/newbound > /tmp/nb-server.log 2>&1 &
timeout 60 bash -c 'until curl -sf -o /dev/null http://127.0.0.1:8080/; do sleep 1; done'
# ... drive it ...
lsof -ti:8080 -sTCP:LISTEN | xargs -r kill    # stop
```

Poll the port; never `sleep 5`. The cwd matters: the binary resolves
`data/` relative to where it starts.

## 5. Driving the UI — the gotchas that cost time

- **Login lives on the home page (`/`), not on app pages.** An app
  page requested unauthenticated serves its static shell (an empty
  `data-control` body) and then just sits there — no login form ever
  appears, no redirect. Log in at `/` first (the session rides a
  cookie), THEN navigate to the app.
- Login form selectors (the `app` library's login facet):
  `#username`, `#password`, submit via `.loginbutton` (an `<a>`, not
  a submit button).
- All app UIs are client-rendered from store facets after `api.js`
  boots — wait for real selectors, never for load events or timers.
- Agent chat selectors (`/agent/index.html`, chat tab): input
  `.ag-input`, send `.ag-send`, conversation `.ag-thread`, tabs
  `.ag-tab[data-tab=...]`, new session `.ag-newsession`.
- **Waiting for an LLM reply:** the thread appends cells for spinner /
  streaming states. Don't count children and stop — poll until the
  LAST cell's text is non-empty AND stable across two consecutive
  polls (see the script). Budget minutes, not seconds.
- Screenshot at every stage and LOOK at them; a blank frame means the
  client JS never booted (check `page.on('console')` errors).

## 6. The driver

`scripts/drive-agent.js` in this skill folder is the verified
end-to-end driver: login at `/`, open agent chat, send a prompt, wait
for a stable reply, screenshot each stage, dump the thread text.

```bash
cd <dir with node_modules/playwright>
BASE_URL=http://127.0.0.1:8080 NB_USER=admin NB_PASS=<password> \
PROMPT="Reply with exactly: QA OK" \
SHOT_DIR=./shots node <this-repo>/.claude/skills/run-ui/scripts/drive-agent.js
```

Optional: `PW_CHROMIUM=/opt/pw-browsers/chromium` when the npm/browser
versions mismatch (see §1). Exit code 0 plus `THREAD_TEXT` on stdout
is the pass signal; screenshots land in `SHOT_DIR`.

Adapt the same skeleton for other apps: keep the login block, swap the
post-login URL and selectors.

## 7. QA hygiene

- Run against a disposable checkout, never a live instance
  (`tools/scratch-instance.md`); UI clicks execute real store commands.
- Nothing this flow creates is committable: `users/*.properties`,
  `runtime/*`, server logs, screenshots.
