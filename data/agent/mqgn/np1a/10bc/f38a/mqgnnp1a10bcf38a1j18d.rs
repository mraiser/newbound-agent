// Playwright-style auto-waiting interaction. One eval CANNOT wait — the
// Noobscape mechanism is JS::Evaluate -> JSON.stringify(rval), fully
// synchronous (a promise rval stringifies to {}), so the actionability wait
// lives HERE in Rust: poll the page with self-contained eval ticks. Each tick
// resolves the selector, checks attached/visible/enabled, and (for
// click/check/uncheck) bounding-box stability across consecutive ticks via a
// window-side breadcrumb keyed by a per-call id; the tick that finds the
// target actionable dispatches in that same synchronous task. Returns
// {status, action, selector, desc, bbox, attempts, elapsed_ms,
// typed?/selected?/already?} or {status:err, msg, state(absent|hidden|
// disabled|stabilizing|no-option|error|channel), attempts, elapsed_ms}.
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

let act_l = action.to_lowercase();
let valid = matches!(act_l.as_str(), "click" | "fill" | "type" | "press" | "select" | "check" | "uncheck");
if !valid {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &format!("unknown action: {} (want click|fill|type|press|select|check|uncheck)", action));
    return o;
}
let needs_arg = matches!(act_l.as_str(), "fill" | "type" | "press" | "select");
if needs_arg && arg.trim().is_empty() {
    let mut o = DataObject::new();
    o.put_string("status", "err");
    o.put_string("msg", &format!("{} requires a non-empty arg", act_l));
    return o;
}
let tmo: i64 = if timeout_ms > 0 { timeout_ms } else { 10000 };
let id = format!("act{}_{}", time(), rand_range(0, 1_000_000));

// One self-contained tick: resolve -> check -> (maybe) dispatch. No re-arm,
// no pending state machine: the tick either dispatches or reports its state,
// and Rust decides whether to tick again.
let js = format!(
    r#"(function(){{
var SL={}, AC={}, AR={}, ID={};
function txt(n){{return (n.textContent||'').replace(/\s+/g,' ').trim();}}
function vis(e){{if(!e||!e.isConnected)return false;var d=e.ownerDocument||document;var r=e.getBoundingClientRect();if(r.width<1||r.height<1)return false;var s=d.defaultView.getComputedStyle(e);return s.display!=='none'&&s.visibility!=='hidden'&&parseFloat(s.opacity||'1')>0;}}
function desc(e){{if(!e)return '';var t=(e.tagName||'').toLowerCase();var id=e.id?('#'+e.id):'';var cls='';try{{if(e.className&&typeof e.className==='string'&&e.className.trim())cls='.'+e.className.trim().split(/\s+/).slice(0,2).join('.');}}catch(x){{}}return t+id+cls;}}
function walk(c,fn,depth){{fn(c);if(depth>24)return;var fr;try{{fr=c.querySelectorAll('iframe,frame');}}catch(x){{return;}}for(var i=0;i<fr.length;i++){{var fd=null;try{{fd=fr[i].contentDocument;}}catch(x){{fd=null;}}if(fd)walk(fd,fn,depth+1);}}}}
function one(root,spec){{
var kind=spec.k,val=spec.v,sub=spec.s,visf=spec.n,nth=spec.i||0;var list=[];
function scanEl(e){{
if(kind==='text'){{if(txt(e).indexOf(val)>=0)list.push(e);}}
else if(kind==='has'){{if(txt(e).toLowerCase().indexOf(val.toLowerCase())>=0)list.push(e);}}
else if(kind==='role'){{if((((e.getAttribute&&e.getAttribute('role'))||'')===val)||((e.tagName||'').toLowerCase()===val.toLowerCase()))list.push(e);}}
else if(kind==='xp'){{try{{var d=e.ownerDocument||document;if(d.evaluate(val,d,null,9,null).singleNodeValue===e)list.push(e);}}catch(x){{}}}}
}}
if(root.nodeType===9){{
walk(root,function(c){{
try{{
if(kind==='css'){{var rr=c.querySelectorAll(val);for(var q=0;q<rr.length;q++)list.push(rr[q]);}}
else{{var all=c.querySelectorAll('body,body *');for(var q2=0;q2<all.length;q2++)scanEl(all[q2]);}}
}}catch(x){{}}
}});
}} else {{
try{{
if(kind==='css'){{if(root.matches&&root.matches(val))list.push(root);var rr2=root.querySelectorAll(val);for(var q3=0;q3<rr2.length;q3++)list.push(rr2[q3]);}}
else{{scanEl(root);var all2=root.querySelectorAll('*');for(var q4=0;q4<all2.length;q4++)scanEl(all2[q4]);}}
}}catch(x){{}}
}}
if(kind==='text'||kind==='has'){{
// prefer the DEEPEST match: a container's text contains every descendant's
// text, so first-match would otherwise resolve text= to <body>. Keep the
// lowest element whose own (sub)tree still matches.
var deep=list.filter(function(e){{return !list.some(function(o){{return o!==e&&e.contains(o);}});}});
if(deep.length>0)list=deep;
}}
if(sub)list=list.filter(function(e){{return txt(e).toLowerCase().indexOf(sub.toLowerCase())>=0;}});
if(visf)list=list.filter(function(e){{return vis(e);}});
return [list,nth];}}
function parse(s){{var steps=[],re=/(?:^|\s*)>>\s*/g,m,last=0;while((m=re.exec(s))){{steps.push(s.slice(last,m.index));last=m.index+m[0].length;}}steps.push(s.slice(last));steps=steps.filter(function(x){{return x.trim().length>0;}});var out=[];
for(var i=0;i<steps.length;i++){{var st=steps[i].trim(),nth=0,nm;
nm=st.match(/^(.*):nth-match\((-?\d+)\)$/);if(nm){{st=nm[1];nth=parseInt(nm[2],10);}}
var visf=false;if(/:visible$/.test(st)){{visf=true;st=st.replace(/:visible$/,'');}}
var mm;
if((mm=st.match(/^text=(.*)$/)))out.push({{k:'text',v:mm[1].trim(),i:nth,n:visf}});
else if((mm=st.match(/^role=(.*)$/)))out.push({{k:'role',v:mm[1].trim(),i:nth,n:visf}});
else if((mm=st.match(/^xpath=(.*)$/)))out.push({{k:'xp',v:mm[1].trim(),i:nth,n:visf}});
else if((mm=st.match(/^:has-text\((.*)\)$/)))out.push({{k:'has',v:mm[1].trim(),i:nth,n:visf}});
else if((mm=st.match(/^css=(.*)$/)))out.push({{k:'css',v:mm[1].trim(),i:nth,n:visf}});
else if((mm=st.match(/^(.*?):has-text\((.*)\)$/)))out.push({{k:'css',v:mm[1].trim(),s:mm[2],i:nth,n:visf}});
else out.push({{k:'css',v:st,i:nth,n:visf}});}}
return out;}}
function resolve(sl){{var steps=parse(sl),bases=[document];
for(var i=0;i<steps.length;i++){{var spec=steps[i],nx=[];
for(var j=0;j<bases.length;j++){{var r=one(bases[j],spec),list=r[0],nth=r[1];
if(list.length>0){{var idx=nth<0?list.length+nth:nth;if(idx<0)idx=0;if(idx>=list.length)idx=list.length-1;nx.push(list[idx]);}}}}
if(nx.length===0)return null;bases=nx;}}
return bases[bases.length-1];}}
function enabled(e){{if(!e)return false;if(e.disabled)return false;if(e.getAttribute&&e.getAttribute('aria-disabled')==='true')return false;var t=(e.tagName||'').toLowerCase();return /^(input|textarea|select|button|a|option)$/.test(t)||e.isContentEditable||(e.tabIndex>=0)||(e.onclick!=null)||((e.getAttribute&&e.getAttribute('role'))!=null);}}
function keydefs(k){{k=String(k);var map={{Enter:['Enter','Enter'],Tab:['Tab','Tab'],Escape:['Escape','Escape'],Backspace:['Backspace','Backspace'],Delete:['Delete','Delete'],Home:['Home','Home'],End:['End','End'],PageUp:['PageUp','PageUp'],PageDown:['PageDown','PageDown'],ArrowLeft:['ArrowLeft','ArrowLeft'],ArrowRight:['ArrowRight','ArrowRight'],ArrowUp:['ArrowUp','ArrowUp'],ArrowDown:['ArrowDown','ArrowDown']}};var names=k.split('+'),out=[];for(var i=0;i<names.length;i++){{var n=names[i].trim();if(map[n])out.push({{key:map[n][0],code:map[n][1]}});else if(n.length===1)out.push({{key:n,code:n,text:n}});}}return out;}}
function kd(e,d,type){{var v=e.ownerDocument.defaultView;try{{e.dispatchEvent(new v.KeyboardEvent(type,{{key:d.key,code:d.code||d.key,bubbles:true,cancelable:true}}));}}catch(x){{var ev=e.ownerDocument.createEvent('Event');ev.initEvent(type,true,true);ev.key=d.key;ev.code=d.code||d.key;e.dispatchEvent(ev);}}}}
function setVal(e,val){{var d=e.ownerDocument,v=d.defaultView;
if(e.isContentEditable){{e.textContent=String(val);}}
else{{var proto=(e.tagName||'').toLowerCase()==='textarea'?(v.HTMLTextAreaElement&&v.HTMLTextAreaElement.prototype):(v.HTMLInputElement&&v.HTMLInputElement.prototype);
var dd=proto&&Object.getOwnPropertyDescriptor(proto,'value');var setter=dd&&dd.set;
if(setter)setter.call(e,String(val));else e.value=String(val);}}
e.dispatchEvent(new v.Event('input',{{bubbles:true}}));e.dispatchEvent(new v.Event('change',{{bubbles:true}}));}}
function doAction(ac,ar,e,o){{var v=e.ownerDocument.defaultView;
if(ac==='click'){{e.click();return true;}}
if(ac==='check'||ac==='uncheck'){{var want=(ac==='check');if(!('checked'in e)){{e.click();return true;}}if(e.checked===want){{o.already=true;return true;}}e.click();return true;}}
if(ac==='select'){{if((e.tagName||'').toLowerCase()!=='select')return 'not a <select>';var want=String(ar).toLowerCase(),hit=-1;for(var i=0;i<e.options.length;i++){{var op=e.options[i];if(op.value===String(ar)||(op.text||'').trim().toLowerCase()===want||(op.label||'').toLowerCase()===want){{hit=i;break;}}}}if(hit<0)return 'no-option: '+ar;e.selectedIndex=hit;e.dispatchEvent(new v.Event('input',{{bubbles:true}}));e.dispatchEvent(new v.Event('change',{{bubbles:true}}));o.selected=e.options[hit]?(e.options[hit].text||e.options[hit].value):'';return true;}}
if(ac==='fill'){{e.focus();setVal(e,ar);o.typed=String(ar);return true;}}
if(ac==='type'){{e.focus();var s=String(ar);for(var i=0;i<s.length;i++){{var ch=s[i];kd(e,{{key:ch,code:ch}},'keydown');try{{if(!e.isContentEditable&&'value'in e)e.value+=ch;else if(e.isContentEditable)e.textContent+=ch;}}catch(x){{}}try{{e.dispatchEvent(new v.InputEvent('input',{{bubbles:true,data:ch,inputType:'insertText'}}));}}catch(x2){{e.dispatchEvent(new v.Event('input',{{bubbles:true}}));}}kd(e,{{key:ch,code:ch}},'keyup');}}o.typed=s;return true;}}
if(ac==='press'){{e.focus();var defs=keydefs(ar),last=null;for(var i=0;i<defs.length;i++){{last=defs[i];kd(e,last,'keydown');if(last.text){{try{{if(!e.isContentEditable&&'value'in e)e.value+=last.text;else if(e.isContentEditable)e.textContent+=last.text;}}catch(x){{}}try{{e.dispatchEvent(new v.InputEvent('input',{{bubbles:true,data:last.text,inputType:'insertText'}}));}}catch(x2){{e.dispatchEvent(new v.Event('input',{{bubbles:true}}));}}}}}}if(last&&last.key==='Enter'){{try{{if(e.form&&e.form.requestSubmit)e.form.requestSubmit();}}catch(x3){{}}}}for(var i2=defs.length-1;i2>=0;i2--)kd(e,defs[i2],'keyup');o.typed=ar;return true;}}
return 'unknown action';}}
var out={{state:'absent',action:AC,selector:SL}};
var e=resolve(SL);
if(!e)return out;
out.desc=desc(e);
var r=e.getBoundingClientRect();out.bbox={{x:r.x,y:r.y,w:r.width,h:r.height}};
if(!vis(e)){{out.state='hidden';return out;}}
if(!enabled(e)){{out.state='disabled';return out;}}
if(AC==='click'||AC==='check'||AC==='uncheck'){{
var st=window.__NB_ACT=window.__NB_ACT||{{}};var p=st[ID];
if(!(p&&p.x===out.bbox.x&&p.y===out.bbox.y&&p.w===out.bbox.w&&p.h===out.bbox.h)){{st[ID]=out.bbox;out.state='stabilizing';return out;}}
delete st[ID];
}}
var res=doAction(AC,AR,e,out);
if(res!==true){{out.state=(String(res).indexOf('no-option')===0)?'no-option':'error';out.msg=String(res);return out;}}
out.state='dispatched';
return out;}})()"#,
    js_string(&selector),
    js_string(&act_l),
    js_string(&arg),
    js_string(&id)
);

let start = time();
let deadline = start + tmo;
let mut attempts: i64 = 0;
let mut last_state = String::from("channel");
let mut last_desc = String::new();
loop {
    attempts += 1;
    let r = crate::agent::browser::eval::eval(js.clone(), 5000);
    let ok = r.try_get_string("status").map(|s| s == "ok").unwrap_or(false);
    if ok {
        if let Ok(v) = r.try_get_object("value") {
            let stt = v.try_get_string("state").unwrap_or_default();
            if let Ok(d) = v.try_get_string("desc") { last_desc = d; }
            match stt.as_str() {
                "dispatched" => {
                    let mut o = DataObject::new();
                    o.put_string("status", "ok");
                    o.put_string("action", &act_l);
                    o.put_string("selector", &selector);
                    if !last_desc.is_empty() { o.put_string("desc", &last_desc); }
                    if let Ok(bb) = v.try_get_object("bbox") { o.put_object("bbox", bb); }
                    if let Ok(t) = v.try_get_string("typed") { o.put_string("typed", &t); }
                    if let Ok(s2) = v.try_get_string("selected") { o.put_string("selected", &s2); }
                    if v.try_get_boolean("already").unwrap_or(false) { o.put_boolean("already", true); }
                    o.put_int("attempts", attempts);
                    o.put_int("elapsed_ms", time() - start);
                    return o;
                }
                "no-option" | "error" => {
                    let mut o = DataObject::new();
                    o.put_string("status", "err");
                    let m = v.try_get_string("msg").unwrap_or_else(|_| stt.clone());
                    o.put_string("msg", &m);
                    o.put_string("state", &stt);
                    o.put_int("attempts", attempts);
                    o.put_int("elapsed_ms", time() - start);
                    return o;
                }
                other => { last_state = other.to_string(); }
            }
        } else {
            last_state = String::from("channel");
        }
    } else {
        last_state = String::from("channel");
    }
    if time() >= deadline { break; }
    std::thread::sleep(std::time::Duration::from_millis(120));
}
let mut o = DataObject::new();
o.put_string("status", "err");
o.put_string("msg", &format!("{} not actionable on `{}` within {} ms (last state: {})", act_l, selector, tmo, last_state));
o.put_string("state", &last_state);
if !last_desc.is_empty() { o.put_string("desc", &last_desc); }
o.put_int("attempts", attempts);
o.put_int("elapsed_ms", time() - start);
o