var me = this;
var ME = $('#' + me.UUID)[0];
me.pollTimer = null;
me.loaded = false;

function U(r){ return (r && typeof r === 'object' && r.data) ? r.data : r; }
function $f(sel){ return $(ME).find(sel); }

me.ready = function(){
  $(ME).on('click', '[data-act]', function(){
    var act = $(this).attr('data-act');
    if (act === 'refresh')      me.refresh();
    else if (act === 'saveconfig') me.saveConfig(this);
    else if (act === 'stage')   me.runStage($(this).attr('data-stage'), this);
    else if (act === 'install') me.doInstall(this);
    else if (act === 'stop')    me.stopBuild(this);
  });
  me.refresh();
};

me.refresh = function(){
  send_builder_status(function(r){
    var d = U(r);
    if (!d || d.status !== 'ok'){ return; }
    var c = d.config || {};
    // Populate config inputs (skip any the user is currently editing).
    $f('input[data-cfg]').each(function(){
      var k = $(this).attr('data-cfg');
      if (!me.loaded || this !== document.activeElement){ this.value = c[k] || ''; }
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
  if (done) li.addClass('done'); else li.removeClass('done');
};

me.updateMonitor = function(running, stage, pid, lastExit, log){
  $f('.nbld-run').toggle(!!running).text(running ? 'running' : '');
  $f('[data-act="stop"]').toggle(!!running);
  var s = '';
  if (running) s = 'stage: ' + (stage||'?') + '  (pid ' + (pid||'?') + ')';
  else if (stage) s = 'last stage: ' + stage + (lastExit !== '' && lastExit != null ? '  → exit ' + lastExit : '');
  $f('.nbld-stage').text(s);
  if (log != null && log !== '') {
    var pre = $f('.nbld-log')[0];
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
      if (!d.running){ me.stopPoll(); me.refresh(); }
    });
  }, 2500);
};
me.stopPoll = function(){ if (me.pollTimer){ clearInterval(me.pollTimer); me.pollTimer = null; } };

me.saveConfig = function(btn){
  var inputs = $f('input[data-cfg]').toArray();
  var msg = $f('.nbld-cfgmsg').text('Saving…').css('color', '#7fd18f');
  $(btn).prop('disabled', true);
  var i = 0;
  (function next(){
    if (i >= inputs.length){
      $(btn).prop('disabled', false);
      msg.text('Saved.');
      setTimeout(function(){ msg.text(''); }, 2500);
      me.refresh();
      return;
    }
    var el = inputs[i++];
    send_set_config($(el).attr('data-cfg'), el.value.trim(), function(r){
      var d = U(r);
      if (d && d.status === 'err'){ msg.text(d.msg || 'error').css('color', '#e08a8a'); }
      next();
    });
  })();
};

me.runStage = function(stage, btn){
  $(btn).prop('disabled', true);
  var done = function(r){
    $(btn).prop('disabled', false);
    var d = U(r);
    if (d && d.status === 'err'){ alert('Noobscape: ' + (d.msg || 'stage failed')); }
    me.refresh();
    if (stage !== 'materialize' && stage !== 'patch') me.startPoll();
  };
  if (stage === 'materialize') send_materialize_kit(done);
  else send_run_stage(stage, done);
};

me.doInstall = function(btn){
  var mode = $f('input[name="nbmode"]:checked').val() || 'symlink';
  var msg = $f('.nbld-installmsg').text('Installing…').css('color', '#7fd18f');
  $(btn).prop('disabled', true);
  send_install(mode, function(r){
    $(btn).prop('disabled', false);
    var d = U(r);
    if (d && d.status === 'ok'){ msg.text(d.msg || 'installed').css('color', '#7fd18f'); }
    else { msg.text((d && d.msg) || 'install failed').css('color', '#e08a8a'); }
    me.refresh();
  });
};

me.stopBuild = function(btn){
  $(btn).prop('disabled', true);
  send_stop_build(function(r){
    $(btn).prop('disabled', false);
    me.stopPoll();
    me.refresh();
  });
};