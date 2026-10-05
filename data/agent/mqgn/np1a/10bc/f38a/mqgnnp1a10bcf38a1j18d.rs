// Playwright-style auto-waiting interaction primitive. One round-trip per
// poll: each eval re-resolves the selector, checks actionability (attached,
// visible, enabled; bounding-box stability for pointer actions), and, when
// actionable, dispatches the action in the SAME synchronous task — the page
// cannot change between check and act. The wait loop lives on the Rust side
// because the v2 transport captures only synchronous return values (a blocked
// in-page wait would freeze the page's own event loop).
//
// Selector engine (beyond CSS):
//   text=Save            innermost element whose text contains "Save"
//   css=div.item         explicit CSS (bare strings are CSS too)
//   :visible             keep only visible matches
//   :nth-match(2)        1-based index into matches
//   form >> text=Submit  chain: descendants of previous matches
//
// Actions: click, fill (native setter, React-safe), type (per-char key
// events), press (Enter/Tab/Escape/arrows/...; Enter requestSubmits),
// select (by value/label/text), check, uncheck.
// Returns {status, action, selector, desc, bbox, attempts, elapsed_ms, ...}
// or {status:err, msg, state(absent|hidden|disabled|stabilizing|no-option),
// attempts}.
fn js_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn errobj(m: &str) -> DataObject {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", m);
    o
}

let action_l = action.trim().to_lowercase();
let known = ["click", "fill", "type", "press", "select", "check", "uncheck"];
if !known.contains(&action_l.as_str()) {
    return errobj(&format!(
        "unknown action '{}' (expected one of: {})",
        action,
        known.join(", ")
    ));
}
let need_stable = matches!(action_l.as_str(), "click" | "check" | "uncheck");
let tmo: i64 = if timeout_ms > 0 { timeout_ms } else { 10000 };

const ENGINE: &str = r##"(function(){
function vis(e){ if(!e||e.nodeType!==1) return false; var w=e.ownerDocument.defaultView; var s=w.getComputedStyle(e); if(s.display==='none'||s.visibility==='hidden'||s.visibility==='collapse') return false; var r=e.getBoundingClientRect(); return r.width>0&&r.height>0; }
function enabled(e){ if(e.disabled) return false; if(e.getAttribute&&e.getAttribute('aria-disabled')==='true') return false; if(e.closest&&e.closest('fieldset[disabled]')) return false; return true; }
function bbox(e){ var r=e.getBoundingClientRect(); return r.left.toFixed(1)+','+r.top.toFixed(1)+','+r.width.toFixed(1)+','+r.height.toFixed(1); }
function desc(e){ var t=e.tagName?e.tagName.toLowerCase():'?'; var id=e.id?'#'+e.id:''; var cn=(''+(e.className||'')).trim().split(/\s+/).filter(Boolean).slice(0,2).map(function(x){return '.'+x;}).join(''); return t+id+cn; }
function norm(s){ return (''+s).replace(/\s+/g,' '); }
function matchSeg(seg,roots){
  var nth=null,needVis=false,m;
  m=seg.match(/:nth-match\((\d+)\)\s*$/); if(m){ nth=parseInt(m[1],10); seg=seg.slice(0,seg.length-m[0].length); }
  m=seg.match(/:nth\((\d+)\)\s*$/); if(m){ nth=parseInt(m[1],10); seg=seg.slice(0,seg.length-m[0].length); }
  if(/:visible\s*$/.test(seg)){ needVis=true; seg=seg.replace(/:visible\s*$/,''); }
  var out=[],mode='css',body=seg;
  if(seg.indexOf('text=')===0){ mode='text'; body=seg.slice(5); }
  else if(seg.indexOf('css=')===0){ body=seg.slice(4); }
  body=body.replace(/^['"]|['"]$/g,'');
  for(var i=0;i<roots.length;i++){
    var root=roots[i];
    if(mode==='text'){
      var needle=norm(body).toLowerCase();
      var all=root.querySelectorAll('*');
      var hits=[];
      for(var j=0;j<all.length;j++){ var el=all[j]; if(norm(el.textContent).toLowerCase().indexOf(needle)>=0) hits.push(el); }
      for(var j2=0;j2<hits.length;j2++){ var h=hits[j2]; var inner=true; for(var j3=0;j3<hits.length;j3++){ if(j3!==j2&&h.contains(hits[j3])){ inner=false; break; } } if(inner) out.push(h); }
    } else {
      try{ var list=root.querySelectorAll(body); for(var k=0;k<list.length;k++) out.push(list[k]); }catch(x){}
    }
  }
  var ded=[]; for(var d=0;d<out.length;d++){ if(ded.indexOf(out[d])<0) ded.push(out[d]); }
  out=ded;
  if(needVis) out=out.filter(vis);
  if(nth!==null) out=(nth>=1&&nth<=out.length)?[out[nth-1]]:[];
  return out;
}
function resolve(sel){
  var segs=sel.split(/\s*>>\s*/);
  var roots=[document];
  var cur=[];
  for(var i=0;i<segs.length;i++){ cur=matchSeg(segs[i],roots); roots=cur; }
  return cur;
}
function nativeSet(e,v){
  var w=e.ownerDocument.defaultView;
  var p;
  if(e instanceof w.HTMLTextAreaElement) p=w.HTMLTextAreaElement.prototype;
  else if(e instanceof w.HTMLSelectElement) p=w.HTMLSelectElement.prototype;
  else p=w.HTMLInputElement.prototype;
  var sd=Object.getOwnPropertyDescriptor(p,'value');
  if(sd&&sd.set) sd.set.call(e,v); else e.value=v;
}
function fire(e,t){ var d=e.ownerDocument,w=d.defaultView,ev; try{ ev=new w.Event(t,{bubbles:true,cancelable:true}); }catch(x){ ev=d.createEvent('Event'); ev.initEvent(t,true,true); } return e.dispatchEvent(ev); }
function keyInfo(k){
  var map={Enter:['Enter','Enter',13],Tab:['Tab','Tab',9],Escape:['Escape','Escape',27],Backspace:['Backspace','Backspace',8],Delete:['Delete','Delete',46],ArrowLeft:['ArrowLeft','ArrowLeft',37],ArrowUp:['ArrowUp','ArrowUp',38],ArrowRight:['ArrowRight','ArrowRight',39],ArrowDown:['ArrowDown','ArrowDown',40],Home:['Home','Home',36],End:['End','End',35],PageUp:['PageUp','PageUp',33],PageDown:['PageDown','PageDown',34],' ':[' ','Space',32]};
  var mm=map[k]; if(mm) return {key:mm[0],code:mm[1],kc:mm[2]};
  if(k&&k.length===1){ var c=k.toUpperCase(); return {key:k,code:'Key'+c,kc:c.charCodeAt(0)}; }
  return null;
}
function keyEv(e,t,ki){ var w=e.ownerDocument.defaultView; var ev; try{ ev=new w.KeyboardEvent(t,{key:ki.key,code:ki.code,bubbles:true,cancelable:true}); }catch(x){ return true; } try{ Object.defineProperty(ev,'keyCode',{get:function(){return ki.kc;}}); Object.defineProperty(ev,'which',{get:function(){return ki.kc;}}); }catch(x2){} return e.dispatchEvent(ev); }
function doAct(action,e,arg){
  try{ e.scrollIntoView({block:'nearest',inline:'nearest'}); }catch(x){ try{ e.scrollIntoView(); }catch(x2){} }
  var tag=e.tagName?e.tagName.toLowerCase():'';
  if(action==='click'){ e.click(); return {}; }
  if(action==='check'||action==='uncheck'){
    var want=(action==='check'); var ty=(''+(e.type||'')).toLowerCase();
    if(ty!=='checkbox'&&ty!=='radio') return {hard:'not a checkbox/radio: '+desc(e)};
    if(!!e.checked===want) return {already:true};
    e.click(); return {};
  }
  if(action==='fill'){
    if(tag==='select') return doAct('select',e,arg);
    if(e.isContentEditable){ e.focus(); e.textContent=arg; fire(e,'input'); return {}; }
    if(e.value===undefined) return {hard:'not editable: '+desc(e)};
    e.focus(); nativeSet(e,arg); fire(e,'input'); fire(e,'change'); return {};
  }
  if(action==='type'){
    if(e.value===undefined&&!e.isContentEditable) return {hard:'not editable: '+desc(e)};
    e.focus();
    for(var i=0;i<arg.length;i++){
      var ch=arg.charAt(i); var ki=keyInfo(ch);
      if(ki) keyEv(e,'keydown',ki);
      if(e.value!==undefined){ nativeSet(e,(''+e.value)+ch); } else { e.textContent=(''+e.textContent)+ch; }
      fire(e,'input');
      if(ki) keyEv(e,'keyup',ki);
    }
    fire(e,'change');
    return {typed:arg.length};
  }
  if(action==='press'){
    var ki2=keyInfo(arg); if(!ki2) return {hard:'unknown key: '+arg};
    e.focus();
    var kd=keyEv(e,'keydown',ki2);
    keyEv(e,'keyup',ki2);
    if(arg==='Backspace'&&kd&&e.value!==undefined){ nativeSet(e,(''+e.value).slice(0,-1)); fire(e,'input'); }
    if(arg==='Enter'&&kd){ var f=e.form; if(f){ try{ if(f.requestSubmit) f.requestSubmit(); else fire(f,'submit'); }catch(x3){} } }
    return {};
  }
  if(action==='select'){
    var sel=(tag==='select')?e:(e.closest?e.closest('select'):null);
    if(!sel) return {hard:'not a select: '+desc(e)};
    var hit=null;
    for(var oi=0;oi<sel.options.length;oi++){ var o=sel.options[oi]; if(o.value===arg||o.label===arg||norm(o.textContent).replace(/^\s+|\s+$/g,'')===arg){ hit=o; break; } }
    if(!hit) return {noopt:true};
    sel.focus(); nativeSet(sel,hit.value); fire(sel,'input'); fire(sel,'change');
    return {selected:hit.value};
  }
  return {hard:'unknown action: '+action};
}
function nb_act(action,sel,arg,prevBbox,needStable){
  var els=resolve(sel);
  if(!els.length) return {done:false,state:'absent'};
  var e=els[0];
  var bb=bbox(e);
  if(!vis(e)) return {done:false,state:'hidden',bbox:bb};
  if(!enabled(e)) return {done:false,state:'disabled',bbox:bb};
  if(needStable&&(!prevBbox||prevBbox!==bb)) return {done:false,state:'stabilizing',bbox:bb};
  var r=doAct(action,e,arg);
  if(r.hard) return {done:true,ok:false,reason:r.hard};
  if(r.noopt) return {done:false,state:'no-option',bbox:bb};
  var out={done:true,ok:true,desc:desc(e),bbox:bbox(e)};
  for(var k in r){ out[k]=r[k]; }
  return out;
}
return nb_act(__ACTION__, __SELECTOR__, __ARG__, __PREV__, __STABLE__);
})()"##;

let start = time();
let deadline = start + tmo;
let mut prev = String::new();
let mut last_state = String::from("absent");
let mut attempts: i64 = 0;

loop {
    let remaining = deadline - time();
    if remaining <= 0 {
        break;
    }
    attempts += 1;
    let prev_js = if prev.is_empty() {
        "null".to_string()
    } else {
        js_string(&prev)
    };
    let js = ENGINE
        .replace("__ACTION__", &js_string(&action_l))
        .replace("__SELECTOR__", &js_string(&selector))
        .replace("__ARG__", &js_string(&arg))
        .replace("__PREV__", &prev_js)
        .replace("__STABLE__", if need_stable { "true" } else { "false" });
    let r = crate::agent::browser::eval::eval(js, remaining.min(5000));
    let ok = r
        .try_get_string("status")
        .map(|s| s == "ok")
        .unwrap_or(false);
    if !ok {
        let m = r
            .try_get_string("msg")
            .unwrap_or_else(|_| "eval failed".to_string());
        return errobj(&format!("act aborted: {}", m));
    }
    let v = match r.try_get_object("value") {
        Ok(v) => v,
        Err(_) => return errobj("act: unexpected eval payload (not an object)"),
    };
    if v.try_get_boolean("done").unwrap_or(false) {
        if v.try_get_boolean("ok").unwrap_or(false) {
            let mut out = DataObject::new();
            out.put_string("status", "ok");
            out.put_string("action", &action_l);
            out.put_string("selector", &selector);
            if let Ok(d) = v.try_get_string("desc") {
                out.put_string("desc", &d);
            }
            if let Ok(b) = v.try_get_string("bbox") {
                out.put_string("bbox", &b);
            }
            if let Ok(s) = v.try_get_string("selected") {
                out.put_string("selected", &s);
            }
            if let Ok(n) = v.try_get_int("typed") {
                out.put_int("typed", n);
            }
            if let Ok(a) = v.try_get_boolean("already") {
                out.put_boolean("already", a);
            }
            out.put_int("attempts", attempts);
            out.put_int("elapsed_ms", time() - start);
            return out;
        }
        let reason = v
            .try_get_string("reason")
            .unwrap_or_else(|_| "action failed".to_string());
        let mut out = errobj(&reason);
        out.put_string("action", &action_l);
        out.put_string("selector", &selector);
        out.put_int("attempts", attempts);
        return out;
    }
    let state = v
        .try_get_string("state")
        .unwrap_or_else(|_| "unknown".to_string());
    last_state = state.clone();
    if state == "stabilizing" {
        prev = v.try_get_string("bbox").unwrap_or_else(|_| String::new());
    } else {
        prev = String::new();
    }
    std::thread::sleep(std::time::Duration::from_millis(120));
}

let mut out = errobj(&format!(
    "act '{}' on '{}' timed out after {} ms (last state: {})",
    action_l, selector, tmo, last_state
));
out.put_string("action", &action_l);
out.put_string("selector", &selector);
out.put_string("state", &last_state);
out.put_int("attempts", attempts);
out