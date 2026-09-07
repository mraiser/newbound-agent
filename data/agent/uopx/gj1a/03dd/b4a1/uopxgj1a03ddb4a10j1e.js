var me = this;
var ME = document.getElementById(me.UUID);
me.pollTimer = null;
me.loaded = false;

function U(r){ return (r && typeof r === 'object' && r.data) ? r.data : r; }
function $f(sel){ return ME.querySelector(sel); }
function $fa(sel){ return Array.from(ME.querySelectorAll(sel)); }
function show(el, on){ if (el) el.style.display = on ? '' : 'none'; }

// Stages that run inline (a text edit) and return their result directly;
// everything else launches detached and is followed in the monitor.
var SYNC_STAGES = { materialize: 1, patch: 1, repatch: 1 };

me.ready = function(){
  ME.addEventListener('click', function(e){
    var btn = e.target.closest('[data-act]');
    if (!btn || !ME.contains(btn)) return;
    var act = btn.getAttribute('data-act');
    if (act === 'refresh')      me.refresh();
    else if (act === 'saveconfig') me.saveConfig(btn);
    else if (act === 'stage')   me.runStage(btn.getAttribute('data-stage'), btn);
    else if (act === 'install') me.doInstall(btn);
    else if (act === 'stop')    me.stopBuild(btn);
  });
  me.refresh();
};

me.refresh = function(){
  send_builder_status(function(r){
    var d = U(r);
    if (!d || d.status !== 'ok'){ return; }
    var c = d.config || {};
    // Populate config inputs (skip any the user is currently editing).
    $fa('input[data-cfg]').forEach(function(el){
      var k = el.getAttribute('data-cfg');
      if (!me.loaded || el !== document.activeElement){ el.value = c[k] || ''; }
    });
    me.loaded = true;
    // Step dots.
    me.mark('materialize', d.kit_present);
    me.mark('extract', d.src_extracted);
    me.mark('patched', d.patched);
    me.mark('deps', d.built);
    me.mark('built', d.built);
    me.updateMonitor(d.build_running, d.stage, d.build_pid, d.last_exit, d.log_tail);
    if (d.build_running && !me.pollTimer) me.startPoll();
  });
};

me.mark = function(step, done){
  var li = $f('li[data-step="' + step + '"]');
  if (li) li.classList.toggle('done', !!done);
};

// The stage message line: every stage answers here, ok or err, so a
// silent no-op (patch answering `already`) can never look like a click
// that did nothing. kind: ok | warn | err.
me.stageMsg = function(text, kind){
  var el = $f('.nbld-stagemsg');
  if (!el) return;
  el.classList.remove('ok', 'warn', 'err');
  if (!text){ el.textContent = ''; show(el, false); return; }
  el.classList.add(kind || 'ok');
  el.textContent = text;
  show(el, true);
};

me.updateMonitor = function(running, stage, pid, lastExit, log){
  var run = $f('.nbld-run');
  show(run, !!running);
  if (run) run.textContent = running ? 'running' : '';
  show($f('[data-act="stop"]'), !!running);
  var s = '';
  if (running) s = 'stage: ' + (stage||'?') + '  (pid ' + (pid||'?') + ')';
  else if (stage) s = 'last stage: ' + stage + (lastExit !== '' && lastExit != null ? '  → exit ' + lastExit : '');
  $f('.nbld-stage').textContent = s;
  if (log != null && log !== '') {
    var pre = $f('.nbld-log');
    var atBottom = pre.scrollHeight - pre.scrollTop - pre.clientHeight < 40;
    pre.textContent = log;
    if (atBottom) pre.scrollTop = pre.scrollHeight;
  }
};

me.startPoll = function(){
  if (me.pollTimer) return;
  me.pollTimer = setInterval(function(){
    send_stage_log(80, function(r){
      var d = U(r);
      if (!d) return;
      me.updateMonitor(d.running, d.stage, d.pid, d.last_exit, d.log_tail);
      if (!d.running){
        me.stopPoll();
        var ex = d.last_exit;
        if (ex !== '' && ex != null){
          if (String(ex) === '0') me.stageMsg('stage ' + (d.stage||'') + ' finished (exit 0).', 'ok');
          else me.stageMsg('stage ' + (d.stage||'') + ' failed (exit ' + ex + ') - see the build log.', 'err');
        }
        me.refresh();
      }
    });
  }, 2500);
};
me.stopPoll = function(){ if (me.pollTimer){ clearInterval(me.pollTimer); me.pollTimer = null; } };

me.saveConfig = function(btn){
  var inputs = $fa('input[data-cfg]');
  var msg = $f('.nbld-cfgmsg');
  msg.textContent = 'Saving…';
  msg.style.color = '#7fd18f';
  btn.disabled = true;
  var i = 0;
  (function next(){
    if (i >= inputs.length){
      btn.disabled = false;
      msg.textContent = 'Saved.';
      setTimeout(function(){ msg.textContent = ''; }, 2500);
      me.refresh();
      return;
    }
    var el = inputs[i++];
    send_set_config(el.getAttribute('data-cfg'), el.value.trim(), function(r){
      var d = U(r);
      if (d && d.status === 'err'){ msg.textContent = d.msg || 'error'; msg.style.color = '#e08a8a'; }
      next();
    });
  })();
};

me.runStage = function(stage, btn){
  btn.disabled = true;
  me.stageMsg('running ' + stage + '…', 'ok');
  var done = function(r){
    btn.disabled = false;
    var d = U(r) || {};
    if (d.status === 'err'){
      me.stageMsg(stage + ': ' + (d.msg || 'stage failed'), 'err');
    } else if (stage === 'patch' && d.action === 'already'){
      me.stageMsg('Source already carries the mechanism - nothing changed. If the v2 block was updated, use Re-patch (or Re-patch & rebuild).', 'warn');
    } else if (stage === 'materialize'){
      me.stageMsg('Build kit materialized from the library assets.', 'ok');
    } else {
      me.stageMsg(d.msg || (stage + ': ok'), 'ok');
    }
    me.refresh();
    if (d.status !== 'err' && !SYNC_STAGES[stage]) me.startPoll();
  };
  if (stage === 'materialize') send_materialize_kit(done);
  else send_run_stage(stage, done);
};

me.doInstall = function(btn){
  var checked = ME.querySelector('input[name="nbmode"]:checked');
  var mode = checked ? checked.value : 'symlink';
  var msg = $f('.nbld-installmsg');
  msg.textContent = 'Installing…';
  msg.style.color = '#7fd18f';
  btn.disabled = true;
  send_install(mode, function(r){
    btn.disabled = false;
    var d = U(r);
    if (d && d.status === 'ok'){ msg.textContent = d.msg || 'installed'; msg.style.color = '#7fd18f'; }
    else { msg.textContent = (d && d.msg) || 'install failed'; msg.style.color = '#e08a8a'; }
    me.refresh();
  });
};

me.stopBuild = function(btn){
  btn.disabled = true;
  send_stop_build(function(r){
    btn.disabled = false;
    me.stopPoll();
    me.stageMsg('build stopped.', 'warn');
    me.refresh();
  });
};