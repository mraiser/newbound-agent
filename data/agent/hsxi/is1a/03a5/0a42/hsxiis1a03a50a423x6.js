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
    lt.title = 'latest step (of ' + (e.step_log ? e.step_log.length : 0) + ') — ' + (e.latest.note || '');
    el.appendChild(lt);
  }

  if (e.detail) {
    var d = document.createElement('div');
    d.className = 'pl-card-detail';
    d.textContent = e.detail;
    d.hidden = true;
    el.appendChild(d);
    el.addEventListener('click', function () { d.hidden = !d.hidden; });
    el.title = 'click to show/hide detail — drag to change lifecycle';
  } else {
    el.title = 'drag to change lifecycle';
  }

  el.addEventListener('dragstart', function (ev) {
    dragClaim = e.claim || '';
    el.classList.add('dragging');
    ev.dataTransfer.setData('text/plain', dragClaim);
    ev.dataTransfer.effectAllowed = 'move';
  });
  el.addEventListener('dragend', function () {
    dragClaim = null;
    el.classList.remove('dragging');
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
  send_board(function (r) {
    if (r && r.status === 'ok') {
      boardHash = r.hash || '';
      render(r.entries || []);
    } else {
      note((r && r.msg) || 'load failed', true);
    }
  });
}

me.ready = function () {
  ME.querySelector('.pl-refresh').addEventListener('click', function () { note(''); load(); });
  ME.querySelector('.pl-add').addEventListener('submit', function (ev) {
    ev.preventDefault();
    var claimEl = ME.querySelector('.pl-add-claim');
    var detailEl = ME.querySelector('.pl-add-detail');
    var claim = claimEl.value.trim();
    if (!claim) return;
    note('filing…');
    send_add_item(claim, detailEl.value.trim(), null, function (r) {
      if (r && r.status === 'ok') { claimEl.value = ''; detailEl.value = ''; note(''); load(); }
      else note((r && r.msg) || 'add failed', true);
    });
  });
  load();
};
