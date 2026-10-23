// plan.js — kanban over kb.plan. The board is a VIEW: columns are the
// lifecycle tokens in each entry's tags (the same tokens epistemic_work
// derives the plan queue from). A drop calls move_item, which rewrites
// that one entry's tags through the journaled patch_control_facet with
// the board's hash as concurrency token — stale_base means someone else
// wrote kb.plan since we loaded, so we reload and ask again.
var me = this;
var ME = document.getElementById(me.UUID);

var LIFECYCLES = ['proposed', 'accepted', 'in-progress', 'completed', 'done', 'abandoned'];
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
var openClaims = {};
var drafts = {}; // claim -> half-typed note text, survives a live re-render

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
  if (probing || loading || !onScreen()) return;
  refreshTimes();
  probing = true;
  send_board_hash(function (r) {
    probing = false;
    if (r && r.status === 'ok' && (r.hash || '') !== boardHash) {
      if (dragClaim !== null || typing()) reloadPending = true; else load();
    }
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
  ta.placeholder = lifecycleOf(e.tags) === 'in-progress'
    ? 'add a note — the agent reads it on its next step (ctrl+enter)'
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
        note(r.lifecycle === 'in-progress' ? 'note added — the agent picks it up on its next step' : 'note added');
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
  ME.querySelector('.pl-refresh').addEventListener('click', function () { note(''); load(); });
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
