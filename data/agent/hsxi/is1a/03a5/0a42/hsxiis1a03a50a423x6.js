// plan.js — kanban over kb.plan. The board is a VIEW: columns are the
// lifecycle tokens in each entry's tags (the same tokens epistemic_work
// derives the plan queue from). A drop calls move_item, which rewrites
// that one entry's tags through the journaled patch_control_facet with
// the board's hash as concurrency token — stale_base means someone else
// wrote kb.plan since we loaded, so we reload and ask again.
var me = this;
var ME = document.getElementById(me.UUID);

// `blocked`: the agent hit a wall it needs a human for (step_item moves the
// card here on STATUS: BLOCKED). The plan loop skips it; drag it back to
// accepted once the blocker is resolved and the agent picks it up again.
var LIFECYCLES = ['proposed', 'accepted', 'in-progress', 'blocked', 'completed', 'done', 'abandoned'];
var boardHash = '';
var dragClaim = null;

// LIVE UPDATE: the board polls the cheap board_hash probe (just kb.plan's
// facet hash, no entries) every POLL_MS while it is actually on screen, and
// reloads the full board only when the hash moves - so a step_item tick, a
// move from another tab, or an add_item lands within a few seconds without
// shipping the whole facet each tick. A re-render that would land mid-drag
// is deferred to dragend; expanded details survive a re-render.
var POLL_MS = 3000;
var pollTimer = null;
var probing = false;
var loading = false;
var reloadPending = false;
var workerStatus = null;
var workerStatusError = '';
var workerChecking = false;
var autonomySaving = false;
var autonomyPending = null;
var openClaims = {};
var drafts = {}; // claim -> half-typed note text, survives a live re-render

// Activity is a separate, instance-local stream. Tool events never alter the
// kb.plan board hash, so a live timeline cannot race card/note edits.
var activityState = { run: null, recent: [], events: [], error: '' };
var activityChecking = false;
var activityCursor = 0;
var activityRunId = '';
var activityView = 'now';
var historyRunId = '';
var historyData = null;
var historyLoading = false;

function activityDuration(ms) {
  var s = Math.max(0, Math.floor((Date.now() - ms) / 1000));
  if (s < 60) return s + 's';
  var m = Math.floor(s / 60);
  return m + 'm ' + (s % 60) + 's';
}

function activityLine(e) {
  if (!e) return '';
  var tool = e.tool ? String(e.tool).replace(/^dev-code-/, '') : '';
  var target = e.target && e.target !== tool ? ' · ' + e.target : '';
  var streak = Number(e.streak || 0) > 1 ? ' · identical failure ' + e.streak + '/3' : '';
  var label = {
    model_wait: 'Waiting for model response',
    model_error: 'Model backend error',
    model_text: 'Model response received',
    tool_requested: 'Tool requested',
    tool_running: 'Tool running',
    tool_result: e.ok ? 'Tool succeeded' : 'Tool failed',
    tool_skipped: 'Duplicate tool call suppressed',
    checkpoint: 'Checkpoint saved',
    finish: 'Run finished',
    start: 'Run started'
  }[e.kind] || e.phase || e.kind || 'Activity';
  return label + (tool ? ': ' + tool : '') + target + streak;
}

function elWith(cls, text) {
  var el = document.createElement('div');
  if (cls) el.className = cls;
  if (text !== undefined && text !== null) el.textContent = String(text);
  return el;
}

function appendKV(host, key, value, cls) {
  var row = elWith('pl-activity-kv');
  row.appendChild(elWith('lbl', key));
  row.appendChild(elWith('value' + (cls ? ' ' + cls : ''), value));
  host.appendChild(row);
  return row;
}

function appendEvents(host, events) {
  var list = elWith('pl-event-list');
  if (!events || !events.length) {
    list.appendChild(elWith('pl-empty', 'No tool activity yet. The run may still be preparing context or waiting for the model.'));
  } else events.forEach(function (e) {
    var row = elWith('pl-event ' + (e.ok === false || /error|failed/.test(e.phase || '') ? 'bad' : ''));
    var h = elWith('pl-event-h');
    var tm = e.time ? new Date(e.time).toLocaleTimeString() : '';
    h.textContent = tm + '  ·  ' + activityLine(e) + (e.round ? '  ·  round ' + e.round : '');
    row.appendChild(h);
    if (e.detail) {
      var d = elWith('pl-event-detail', e.detail);
      d.title = e.kind === 'tool_requested' ? 'Call id; raw arguments are not shown in the live stream.' : '';
      row.appendChild(d);
    }
    list.appendChild(row);
  });
  host.appendChild(list);
}

function renderHandoff(host, run) {
  var h = run && run.handoff || {};
  var c = run && run.context || {};
  var manifest = elWith('pl-manifest');
  manifest.appendChild(elWith('pl-subhead', 'Context supplied at turn start'));
  appendKV(manifest, 'goal / detail', c.detail_policy === 'full' ? 'Full goal and detail (' + (c.detail_chars || 0) + ' chars)' : 'See plan item');
  appendKV(manifest, 'notes', (c.notes_presented || 0) + ' included · ' + (c.notes_new || 0) + ' new');
  appendKV(manifest, 'prior context', (c.prior_chars || 0) + ' chars · ' + (c.checkpoint_policy || 'latest checkpoint'));
  appendKV(manifest, 'history', c.history_policy || 'Bounded recent steps; run history available on demand');
  host.appendChild(manifest);
  host.appendChild(elWith('pl-subhead', 'Checkpoint available for resumption'));
  host.appendChild(elWith('pl-checkpoint', h.checkpoint || run.checkpoint ||
    'No checkpoint yet. It is saved when the turn reaches a checkpoint/finish boundary; the live activity above shows work already observed.'));
  if (h.next) { host.appendChild(elWith('pl-subhead', 'Next action')); host.appendChild(elWith('pl-checkpoint-next', h.next)); }
  var note = elWith('pl-continuity-note', 'During this turn the agent has its live conversation. The next turn receives this bounded checkpoint plus the goal, notes, and recent step evidence—not an invisible replay of every raw message. It can inspect this run’s activity when it needs more detail.');
  host.appendChild(note);
}

function renderActivity() {
  var panel = ME.querySelector('.pl-activity');
  if (!panel) return;
  var a = activityState;
  var run = a.run || a.recent[0] || null;
  panel.hidden = !run && !a.error;
  var title = ME.querySelector('.pl-activity-title');
  var updated = ME.querySelector('.pl-activity-updated');
  title.textContent = a.error ? 'Activity unavailable: ' + a.error :
    run ? ((run.active ? 'Working: ' : (run.stale ? 'Interrupted: ' : 'Latest run: ')) + (run.claim || run.run_id)) : 'No plan runs recorded yet';
  updated.textContent = run && run.updated ? 'last event ' + relTime(run.updated) : '';
  panel.querySelectorAll('[data-aview]').forEach(function (b) {
    b.classList.toggle('on', b.dataset.aview === activityView);
  });
  var body = ME.querySelector('.pl-activity-body');
  body.textContent = '';
  if (!run) { body.appendChild(elWith('pl-empty', a.error || 'Start an accepted plan item to see its live activity here.')); return; }
  if (activityView === 'history') {
    if (historyLoading) { body.appendChild(elWith('pl-empty', 'Loading run history…')); return; }
    if (historyData && historyData.status === 'err') { body.appendChild(elWith('pl-empty', historyData.msg)); return; }
    if (historyData) {
      body.appendChild(elWith('pl-subhead', (historyData.active ? 'Active' : historyData.stale ? 'Interrupted' : 'Finished') + ' run · ' + (historyData.run_id || '')));
      appendEvents(body, historyData.events || []);
      var hh = elWith('pl-history-handoff');
      renderHandoff(hh, historyData); body.appendChild(hh);
      return;
    }
  }
  if (activityView === 'handoff') { renderHandoff(body, run); return; }
  if (activityView === 'activity') { appendEvents(body, a.events); return; }
  if (activityView === 'history') { body.appendChild(elWith('pl-empty', 'Choose a recent run below.')); }
  var now = elWith('pl-now-grid');
  appendKV(now, 'state', run.stale ? 'interrupted — no live worker owns this old run' : (run.phase || run.state || 'preparing'), run.stale ? 'bad' : '');
  appendKV(now, 'turn / round', 'turn ' + (run.turn || '—') + ' · round ' + (run.round || 0) + '/' + (run.max_rounds || '—'));
  appendKV(now, 'tools / errors', (run.tool_calls || 0) + ' executed · ' + (run.errors || 0) + ' failures' + (run.failure_streak ? ' · current identical streak ' + run.failure_streak + '/3' : ''));
  appendKV(now, 'elapsed', run.started ? activityDuration(run.started) : '—');
  appendKV(now, 'last event', run.updated ? relTime(run.updated) : '—');
  body.appendChild(now);
  if (run.pending_tool && run.pending_tool.tool) appendKV(body, 'pending tool', run.pending_tool.tool + ' · ' + (run.pending_tool.target || ''));
  var last = a.events[a.events.length - 1];
  if (last) {
    body.appendChild(elWith('pl-subhead', 'Latest observed activity'));
    appendEvents(body, [last]);
  } else if (!run.active) {
    body.appendChild(elWith('pl-checkpoint-next', run.outcome ? 'Outcome: ' + run.outcome : 'Open activity or handoff to inspect this run.'));
  }
  if (a.recent.length) {
    body.appendChild(elWith('pl-subhead', 'Recent turns — open activity or handoff'));
    var rr = elWith('pl-run-list');
    a.recent.forEach(function (r) {
      var row = document.createElement('button'); row.type = 'button'; row.className = 'pl-run-row';
      row.textContent = (r.active ? '● ' : r.stale ? '⚠ ' : '') + (r.claim || r.run_id) + ' · turn ' + (r.turn || '?') + ' · ' + (r.outcome || r.phase || '');
      row.addEventListener('click', function () {
        historyRunId = r.run_id; historyData = null; historyLoading = true; activityView = 'history';
        loadHistory(); renderActivity();
      }); rr.appendChild(row);
    }); body.appendChild(rr);
  }
}

function activityEnvelope(r) { return r && r.data && typeof r.data === 'object' ? r.data : r; }

function probeActivity() {
  if (activityChecking || !onScreen()) return;
  activityChecking = true;
  send_activity_status(activityCursor, 200, function (raw) {
    activityChecking = false;
    var r = activityEnvelope(raw);
    if (!r || r.status !== 'ok') {
      activityState.error = (r && r.msg) || 'could not read run activity';
      renderActivity(); return;
    }
    activityState.error = '';
    activityState.recent = r.recent || [];
    var next = r.run || null;
    var nextId = next && next.run_id || '';
    if (nextId !== activityRunId) { activityCursor = 0; activityState.events = []; activityRunId = nextId; }
    if (next) {
      activityState.run = next;
      activityCursor = next.seq || activityCursor;
      (r.events || []).forEach(function (e) {
        if (!activityState.events.some(function (x) { return x.seq === e.seq; })) activityState.events.push(e);
      });
      if (activityState.events.length > 400) activityState.events = activityState.events.slice(-400);
    } else { activityState.run = null; activityCursor = 0; activityState.events = []; activityRunId = ''; }
    if (activityView === 'history' && historyRunId && historyData && historyData.active) loadHistory();
    renderActivity();
  });
}

function loadHistory() {
  if (!historyRunId) return;
  send_activity_history(historyRunId, 0, 400, function (raw) {
    historyLoading = false; historyData = activityEnvelope(raw); renderActivity();
  });
}

// a note textarea with focus defers live reloads (a re-render would steal it)
function typing() {
  var a = document.activeElement;
  return !!(a && a.classList && a.classList.contains('pl-noteform-text') && ME.contains(a));
}

function onScreen() {
  if (!ME || !ME.isConnected) return false;
  if (document.visibilityState === 'hidden') return false;
  return ME.offsetParent !== null || ME.getClientRects().length > 0;
}

function refreshTimes() {
  var lts = ME.querySelectorAll('.pl-card-latest[data-time]');
  for (var i = 0; i < lts.length; i++) {
    var lt = lts[i];
    var t = Number(lt.dataset.time);
    if (!t) continue;
    lt.textContent = lt.dataset.st + ' · ' + relTime(t) + ' — ' + lt.dataset.note;
  }
}

function probe() {
  if (!ME.isConnected) { if (pollTimer) clearInterval(pollTimer); pollTimer = null; return; }
  if (!onScreen()) return;
  refreshTimes();
  probeWorker();
  if (probing || loading) return;
  probing = true;
  send_board_hash(function (r) {
    probing = false;
    if (r && r.status === 'ok' && (r.hash || '') !== boardHash) {
      if (dragClaim !== null || typing()) reloadPending = true; else load();
    }
  });
}

function isRunning(r) {
  return !!(r && r.plan_autonomy && r.phase === 'stepping' && r.current);
}

function claimList(a) {
  var list = (a || []).slice(0, 2);
  var extra = (a || []).length - list.length;
  return list.map(function (c) { return '“' + c + '”'; }).join(', ') +
    (extra > 0 ? ' and ' + extra + ' more' : '');
}

// Explain the WORKER's state, not just a card's lifecycle. In particular,
// in-progress means unfinished, not necessarily executing at this instant.
function workerText(r) {
  var phase = r.phase || 'unknown';
  if (isRunning(r)) return 'Running now: ' + r.current;
  var prefix = 'No task running: ';
  if (phase === 'off' && r.alive) return prefix + 'autonomy was just enabled; waiting for the worker to start.';
  if (!r.alive) return prefix + 'autonomy is enabled, but its worker is not running. Start the plan worker to run tasks.';
  if (phase === 'starting') return prefix + 'the plan worker is starting.';
  if (phase === 'rate_limited') {
    var resume = r.resume_at ? ' Resumes ' + new Date(r.resume_at).toLocaleTimeString() + '.' : '';
    return prefix + 'the hourly plan-step limit was reached (' + (r.turns_last_hour || 0) +
      '/' + (r.per_hour || 0) + ').' + resume;
  }
  if (phase === "waiting for the previous worker's step") {
    return prefix + "the previous worker's step is still finishing" + (r.next ? '; next: ' + r.next : '') + '.';
  }
  if (phase === 'idle') {
    if (r.awaiting_reapproval && r.awaiting_reapproval.length) {
      return prefix + claimList(r.awaiting_reapproval) + ' reached the ' + r.max_steps +
        '-step limit. Move the item back to accepted to re-approve it.';
    }
    if (r.parked && r.parked.length) {
      var p = r.parked[0];
      var why = p.reason || 'waiting for a change';
      var retry = p.until > 0 ? '; automatic retry ' + new Date(p.until).toLocaleTimeString() :
        '; change the item or add a note to retry';
      var failure = r.last && r.last.action === 'step_failed' && r.last.claim === p.claim && r.last.msg ?
        ' Last error: ' + r.last.msg : '';
      return prefix + (r.parked.length === 1 ? 'one item is parked' : r.parked.length + ' items are parked') +
        '. ' + p.claim + ' — ' + why + retry + '.' + failure;
    }
    if (r.waiting_for_children && r.waiting_for_children.length) {
      return prefix + 'parent work is waiting for active children: ' + claimList(r.waiting_for_children) + '.';
    }
    if (!r.eligible) return prefix + 'there are no eligible accepted or in-progress tasks. Accept a task to start work.';
    return prefix + 'the next eligible task is being prepared' + (r.next ? ': ' + r.next : '') + '.';
  }
  if (phase === 'stepping') return prefix + 'the worker is between steps; the next task should start shortly.';
  return prefix + 'worker phase: ' + phase + (r.next ? '. Next: ' + r.next : '') + '.';
}

function renderWorker() {
  var bar = ME.querySelector('.pl-autonomy');
  var running = isRunning(workerStatus);
  var enabled = !!(workerStatus && workerStatus.plan_autonomy);
  var checkbox = ME.querySelector('.pl-autonomy-checkbox');
  var shown = autonomySaving ? autonomyPending : enabled;
  checkbox.checked = shown === true;
  checkbox.disabled = autonomySaving;
  checkbox.setAttribute('aria-busy', autonomySaving ? 'true' : 'false');
  ME.querySelector('.pl-autonomy-label').textContent = autonomySaving ?
    (autonomyPending ? 'enabling autonomy…' : 'disabling autonomy…') :
    'autonomy ' + (enabled ? 'on' : 'off');
  checkbox.title = enabled ? 'Disable plan autonomy — prevents new steps; a running step finishes.' :
    'Enable plan autonomy — persists the setting and starts the plan worker.';
  bar.hidden = !enabled && !workerStatusError;
  bar.classList.toggle('running', running);
  bar.classList.toggle('error', !!workerStatusError);
  ME.querySelector('.pl-autonomy-text').textContent = workerStatusError ||
    (enabled ? workerText(workerStatus) : '');
  var cards = ME.querySelectorAll('.pl-card');
  for (var i = 0; i < cards.length; i++) {
    var active = running && cards[i].dataset.claim === workerStatus.current;
    cards[i].classList.toggle('running', active);
    if (active) cards[i].setAttribute('aria-current', 'true');
    else cards[i].removeAttribute('aria-current');
    var badge = cards[i].querySelector(':scope > .pl-card-running');
    if (badge) {
      badge.hidden = !active;
      badge.textContent = active ? 'running now · plan autonomy' : '';
    }
  }
}

function probeWorker() {
  if (workerChecking) return;
  workerChecking = true;
  send_worker_status(function (r) {
    workerChecking = false;
    if (r && r.status === 'ok') {
      // worker_status returns JSONObject; board/board_hash return FLAT.
      // Accept either envelope so a failed/flat transport never hides status.
      workerStatus = r.data && typeof r.data === 'object' ? r.data : r;
      workerStatusError = '';
    } else {
      workerStatus = null; // never leave an old running highlight on a failed read
      workerStatusError = 'Cannot read plan-autonomy status: ' + ((r && r.msg) || 'worker status request failed') + '.';
    }
    if (ME.isConnected) renderWorker();
  });
}

function setAutonomy(enabled) {
  if (autonomySaving) return;
  autonomySaving = true;
  autonomyPending = enabled;
  note(enabled ? 'enabling plan autonomy…' : 'disabling plan autonomy…');
  renderWorker();
  send_set_autonomy(enabled, function (r) {
    if (!ME.isConnected) return;
    autonomySaving = false;
    autonomyPending = null;
    if (r && r.status === 'ok') {
      var saved = r.data && typeof r.data === 'object' ? r.data : r;
      workerStatus = Object.assign({}, workerStatus || {}, {
        plan_autonomy: saved.plan_autonomy === true
      });
      note(enabled ? 'plan autonomy enabled' : 'plan autonomy disabled — no new steps');
      probeWorker();
    } else {
      // A failure does not invent an enabled/disabled state: re-read it.
      note((r && r.msg) || 'could not change plan autonomy', true);
      workerStatusError = 'Could not change plan autonomy: ' + ((r && r.msg) || 'request failed') + '.';
      probeWorker();
    }
    renderWorker();
  });
}

function note(txt, isErr) {
  var n = ME.querySelector('.pl-note');
  n.textContent = txt || '';
  n.classList.toggle('err', !!isErr);
}

function lifecycleOf(tags) {
  var toks = String(tags || '').split(',').map(function (t) { return t.trim(); });
  for (var i = 0; i < LIFECYCLES.length; i++) {
    if (toks.indexOf(LIFECYCLES[i]) >= 0) return LIFECYCLES[i];
  }
  return 'proposed'; // untagged entries surface here; a move normalizes them
}

function relTime(ms) {
  var d = Date.now() - ms;
  if (d < 0) d = 0;
  var m = Math.floor(d / 60000);
  if (m < 1) return 'just now';
  if (m < 60) return m + 'm ago';
  var h = Math.floor(m / 60);
  if (h < 24) return h + 'h ago';
  return Math.floor(h / 24) + 'd ago';
}

function card(e) {
  var el = document.createElement('div');
  el.className = 'pl-card';
  el.draggable = true;
  el.dataset.claim = e.claim || '';

  var runningBadge = document.createElement('div');
  runningBadge.className = 'pl-card-running';
  runningBadge.hidden = true;
  el.appendChild(runningBadge);

  var c = document.createElement('div');
  c.className = 'pl-card-claim';
  c.textContent = e.claim || '(no claim)';
  el.appendChild(c);

  var meta = document.createElement('div');
  meta.className = 'pl-card-meta';
  var bits = [];
  if (e.confidence) bits.push(e.confidence);
  if (e.time) bits.push(new Date(e.time).toISOString().slice(0, 10));
  var extra = String(e.tags || '').split(',').map(function (t) { return t.trim(); })
    .filter(function (t) { return t && t !== 'plan' && LIFECYCLES.indexOf(t) < 0; });
  if (extra.length) bits.push(extra.join(' · '));
  meta.textContent = bits.join('  ·  ');
  el.appendChild(meta);

  // rolling-latest: the agent's most recent step headline (written by step_item
  // each tick). The live pulse of the card - progress visible without opening
  // step_log. Show status + relative time + the note.
  if (e.latest && e.latest.note) {
    var lt = document.createElement('div');
    lt.className = 'pl-card-latest';
    var when = e.latest.time ? relTime(e.latest.time) : '';
    var st = e.latest.status ? e.latest.status.toLowerCase().replace(/_/g, ' ') : '';
    lt.textContent = (st ? st : 'update') + (when ? ' · ' + when : '') + ' — ' + e.latest.note;
    if (e.latest.time) {
      lt.dataset.time = String(e.latest.time);
      lt.dataset.st = st ? st : 'update';
      lt.dataset.note = e.latest.note;
    }
    lt.title = 'latest step (of ' + (e.step_log ? e.step_log.length : 0) + ') — ' + (e.latest.note || '');
    el.appendChild(lt);
  }

  // NOTES (agent.plan.add_note): the human's running guidance on an item.
  // The count chip is always visible; an unseen note (no seen_at) is one the
  // agent has not been shown yet - step_item flags it NEW in the next step
  // prompt and stamps seen_at after that turn.
  var key = e.claim || '';
  var notes = Array.isArray(e.notes) ? e.notes.filter(function (n) { return n && n.text; }) : [];
  var unseen = notes.filter(function (n) { return !n.seen_at; }).length;
  if (notes.length) {
    var nc = document.createElement('div');
    nc.className = 'pl-card-notecount' + (unseen ? ' new' : '');
    nc.textContent = '✎ ' + notes.length + (notes.length === 1 ? ' note' : ' notes') +
      (unseen ? ' · ' + unseen + ' not yet seen by the agent' : '');
    el.appendChild(nc);
  }

  // the expandable body: detail + notes + the add-note form
  var body = document.createElement('div');
  body.className = 'pl-card-body';
  body.hidden = !openClaims[key];
  if (e.detail) {
    var d = document.createElement('div');
    d.className = 'pl-card-detail';
    d.textContent = e.detail;
    body.appendChild(d);
  }
  if (notes.length) {
    var nl = document.createElement('div');
    nl.className = 'pl-notes';
    notes.forEach(function (n) {
      var row = document.createElement('div');
      row.className = 'pl-notes-row' + (n.seen_at ? '' : ' new');
      var nh = document.createElement('div');
      nh.className = 'pl-notes-h';
      nh.textContent = (n.author || 'owner') + (n.time ? ' · ' + relTime(n.time) : '') + ' · ' +
        (n.seen_at ? 'seen by the agent ' + relTime(n.seen_at) : 'new — rides the next step');
      var nt = document.createElement('div');
      nt.className = 'pl-notes-t';
      nt.textContent = n.text;
      row.appendChild(nh);
      row.appendChild(nt);
      nl.appendChild(row);
    });
    body.appendChild(nl);
  }
  var nf = document.createElement('form');
  nf.className = 'pl-noteform';
  var ta = document.createElement('textarea');
  ta.className = 'pl-noteform-text';
  ta.rows = 2;
  var lc = lifecycleOf(e.tags);
  ta.placeholder = lc === 'in-progress'
    ? 'add a note — the agent reads it on its next step (ctrl+enter)'
    : lc === 'blocked'
      ? 'answer the blocker here, then drag the card back to accepted (ctrl+enter)'
      : 'add a note to this item (ctrl+enter)';
  ta.setAttribute('aria-label', 'Add a note to this plan item');
  ta.value = drafts[key] || '';
  var nb = document.createElement('button');
  nb.type = 'submit';
  nb.textContent = '+ note';
  nf.appendChild(ta);
  nf.appendChild(nb);
  body.appendChild(nf);
  el.appendChild(body);

  // typing inside a draggable card: suspend drag while the textarea has focus
  // (otherwise text selection turns into a card drag), keep the draft across
  // re-renders, and defer live reloads until the textarea lets go.
  ['click', 'mousedown', 'dragstart'].forEach(function (t) {
    nf.addEventListener(t, function (ev) { ev.stopPropagation(); });
  });
  ta.addEventListener('focus', function () { el.draggable = false; });
  ta.addEventListener('blur', function () {
    el.draggable = true;
    if (reloadPending && dragClaim === null) { reloadPending = false; setTimeout(load, 0); }
  });
  ta.addEventListener('input', function () {
    if (ta.value) drafts[key] = ta.value; else delete drafts[key];
  });
  ta.addEventListener('keydown', function (ev) {
    if (ev.key === 'Enter' && (ev.ctrlKey || ev.metaKey)) { ev.preventDefault(); nf.requestSubmit ? nf.requestSubmit() : nb.click(); }
  });
  nf.addEventListener('submit', function (ev) {
    ev.preventDefault();
    var txt = ta.value.trim();
    if (!txt) return;
    nb.disabled = true;
    note('adding note…');
    send_add_note(key, txt, null, function (r) {
      nb.disabled = false;
      if (r && r.status === 'ok') {
        delete drafts[key];
        ta.value = '';
        openClaims[key] = true;
        note(r.lifecycle === 'in-progress' ? 'note added — the agent picks it up on its next step'
          : r.lifecycle === 'blocked' ? 'note added — drag the card back to accepted to unblock it'
          : 'note added');
        load();
      } else {
        note((r && r.msg) || 'add note failed', true);
      }
    });
  });

  el.addEventListener('click', function () {
    body.hidden = !body.hidden;
    if (body.hidden) delete openClaims[key]; else openClaims[key] = true;
  });
  el.title = 'click to show/hide ' + (e.detail ? 'detail & ' : '') + 'notes — drag to change lifecycle';

  el.addEventListener('dragstart', function (ev) {
    dragClaim = e.claim || '';
    el.classList.add('dragging');
    ev.dataTransfer.setData('text/plain', dragClaim);
    ev.dataTransfer.effectAllowed = 'move';
  });
  el.addEventListener('dragend', function () {
    dragClaim = null;
    el.classList.remove('dragging');
    if (reloadPending) { reloadPending = false; setTimeout(load, 0); }
  });
  return el;
}

function render(entries) {
  var board = ME.querySelector('.pl-board');
  board.textContent = '';
  var byCol = {};
  LIFECYCLES.forEach(function (l) { byCol[l] = []; });
  (entries || []).forEach(function (e) {
    if (!e || !e.claim) return; // skip the line-format terminator object
    byCol[lifecycleOf(e.tags)].push(e);
  });
  LIFECYCLES.forEach(function (l) {
    byCol[l].sort(function (a, b) { return (b.time || 0) - (a.time || 0); });
    var col = document.createElement('div');
    col.className = 'pl-col';
    col.dataset.lifecycle = l;

    var h = document.createElement('div');
    h.className = 'pl-col-h';
    h.innerHTML = '<span></span><span></span>';
    h.firstChild.textContent = l;
    h.lastChild.textContent = byCol[l].length;
    col.appendChild(h);

    var cards = document.createElement('div');
    cards.className = 'pl-col-cards';
    byCol[l].forEach(function (e) { cards.appendChild(card(e)); });
    col.appendChild(cards);

    col.addEventListener('dragover', function (ev) {
      if (dragClaim === null) return;
      ev.preventDefault();
      ev.dataTransfer.dropEffect = 'move';
      col.classList.add('over');
    });
    col.addEventListener('dragleave', function () { col.classList.remove('over'); });
    col.addEventListener('drop', function (ev) {
      ev.preventDefault();
      col.classList.remove('over');
      var claim = ev.dataTransfer.getData('text/plain') || dragClaim;
      if (!claim) return;
      note('moving to ' + l + '…');
      send_move_item(claim, l, boardHash, null, function (r) {
        if (r && r.status === 'ok') { note(''); load(); }
        else if (r && r.msg === 'stale_base') {
          note('kb.plan changed underneath — reloaded, drop again', true);
          load();
        } else {
          note((r && r.msg) || 'move failed', true);
        }
      });
    });
    board.appendChild(col);
  });
  renderWorker();
}

function load() {
  if (loading) { reloadPending = true; return; }
  loading = true;
  send_board(function (r) {
    loading = false;
    if (r && r.status === 'ok') {
      boardHash = r.hash || '';
      render(r.entries || []);
    } else {
      note((r && r.msg) || 'load failed', true);
    }
    if (reloadPending && dragClaim === null) { reloadPending = false; load(); }
  });
}

function startLive() {
  if (!pollTimer) pollTimer = setInterval(probe, POLL_MS);
  // the plan pane starts hidden inside the chat's tab strip: probe the moment
  // it (or the browser tab) comes back on screen instead of waiting a tick
  if (window.IntersectionObserver) {
    new IntersectionObserver(function (ents) {
      for (var i = 0; i < ents.length; i++) if (ents[i].isIntersecting) { probe(); break; }
    }).observe(ME);
  }
  document.addEventListener('visibilitychange', function () {
    if (document.visibilityState === 'visible') probe();
  });
}

me.ready = function () {
  startLive();
  ME.querySelector('.pl-autonomy-checkbox').addEventListener('change', function () {
    setAutonomy(this.checked);
  });
  ME.querySelector('.pl-add').addEventListener('submit', function (ev) {
    ev.preventDefault();
    var claimEl = ME.querySelector('.pl-add-claim');
    var detailEl = ME.querySelector('.pl-add-detail');
    var claim = claimEl.value.trim();
    if (!claim) return;
    note('filing…');
    send_add_item(claim, detailEl.value.trim(), '', null, function (r) {
      if (r && r.status === 'ok') { claimEl.value = ''; detailEl.value = ''; note(''); load(); }
      else note((r && r.msg) || 'add failed', true);
    });
  });
  load();
};
