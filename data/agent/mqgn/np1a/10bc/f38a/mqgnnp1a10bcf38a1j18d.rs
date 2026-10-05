// Playwright-style auto-waiting interaction: wait (in ONE page-side eval) until
// the target is attached AND visible AND enabled, and (for click/check/uncheck)
// bounding-box stable, then dispatch `action`. The whole wait runs inside the
// page: the eval re-arms itself with setTimeout(<id>, IVL) — the Noobscape
// mechanism evaluates the request file's CURRENT source each poll (the dedup
// latch suppresses a response rewrite, NOT re-execution), so the armed timeout
// re-runs this same source against the current document and reports on the
// terminal pass — one file round-trip, instead of N. Returns a structured
// object {status, action, selector, desc, bbox, attempts, elapsed_ms, ...}
// or {status:err, msg, state}.
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
// The page-side poll interval the script re-arms with; the eval channel must
// outlive at least one IVL cycle to ever see a terminal result.
let interval_ms: i64 = 250;
let channel = tmo * 3 + 15000;

let js = format!(
    r#"(function(){{
var SL={}, AC={}, AR={}, TMO={}, IVL={}, T0=Date.now();
function txt(n){{return (n.textContent||'').replace(/\s+/g,' ').trim();}}
function vis(e){{if(!e||!e.isConnected)return false;var d=e.ownerDocument||document;var r=e.getBoundingClientRect();if(r.width<1||r.height<1)return false;var s=d.defaultView.getComputedStyle(e);return s.display!=='none'&&s.visibility!=='hidden'&&parseFloat(s.opacity||'1')>0;}}
function desc(e){{if(!e)return '';var t=(e.tagName||'').toLowerCase();var id=e.id?'#'+e.id:'';var cls=(e.className&&e.className.baseVal===undefined)?('.'+String(e.className).trim().split(/\s+/).slice(0,2).join('.')):'';return t+id+cls;}}
function walk(c,fn,depth){{fn(c);if(depth>24)return;var fr;try{{fr=c.querySelectorAll('iframe,frame');}}catch(x){{return;}}for(var i=0;i<fr.length;i++){{var fd=null;try{{fd=fr[i].contentDocument;}}catch(x){{fd=null;}}if(fd)walk(fd,fn,depth+1);}}}}
function one(doc,base,spec){{var kind=spec.k,val=spec.v,sub=spec.s,visf=spec.n,nth=spec.i||0;
if(kind==='css'){{var out=[];walk(doc,function(c){{try{{var r=c.querySelectorAll(base);for(var j=0;j<r.length;j++)out.push([r[j],c]);}}catch(x){{}}}});if(sub)out=out.filter(function(p){{return txt(p[0]).toLowerCase().indexOf(sub.toLowerCase())>=0;}});if(visf)out=out.filter(function(p){{return vis(p[0]);}});return [out,nth];}}
var list=[];walk(doc,function(c){{var a;try{{a=c.querySelectorAll('body,body *');}}catch(x){{a=[];}}for(var j=0;j<a.length;j++){{var e=a[j];if(kind==='text'&&txt(e).indexOf(val)>=0)list.push([e,c]);else if(kind==='has'&&txt(e).toLowerCase().indexOf(val.toLowerCase())>=0)list.push([e,c]);else if(kind==='role'&&(((e.getAttribute&&e.getAttribute('role'))||'')===val||((e.tagName||'').toLowerCase()===val.toLowerCase())))list.push([e,c]);else if(kind==='xp'){{try{{if(c.evaluate(val,c,null,9,null).singleNodeValue===e)list.push([e,c]);}}catch(x){{}}}}}}}});if(visf)list=list.filter(function(p){{return vis(p[0]);}});return [list,nth];}}
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
function resolve(sl){{var steps=parse(sl),cs=[document];for(var i=0;i<steps.length;i++){{var nx=[],spec=steps[i];
for(var j=0;j<cs.length;j++){{var base=cs[j];var doc=(base.getRootNode?base.getRootNode():document);doc=(doc&&doc.nodeType===9)?doc:(base.ownerDocument||document);
var r=one(doc,spec.v,spec),list=r[0],nth=r[1];
if(base!==document&&base.querySelectorAll&&spec.k==='css'){{try{{var rr=base.querySelectorAll(spec.v);for(var q=0;q<rr.length;q++)list.push([rr[q],doc]);}}catch(x){{}}}}
if(list.length>0){{var idx=nth<0?list.length+nth:nth;if(idx<0)idx=0;if(idx>=list.length)idx=list.length-1;nx.push(list[idx][0]);}}}}
if(nx.length===0)return null;cs=nx;}}
return cs[cs.length-1];}}
function enabled(e){{if(!e)return false;if(e.disabled)return false;if(e.getAttribute&&e.getAttribute('aria-disabled')==='true')return false;var t=(e.tagName||'').toLowerCase();return /^(input|textarea|select|button|a|option)$/.test(t)||e.isContentEditable||(e.tabIndex>=0)||(e.onclick!=null)||((e.getAttribute&&e.getAttribute('role'))!=null);}}
function keydefs(k){{k=String(k);var map={{Enter:['Enter','Enter'],Tab:['Tab','Tab'],Escape:['Escape','Escape'],Backspace:['Backspace','Backspace'],Delete:['Delete','Delete'],Home:['Home','Home'],End:['End','End'],PageUp:['PageUp','PageUp'],PageDown:['PageDown','PageDown'],ArrowLeft:['ArrowLeft','ArrowLeft'],ArrowRight:['ArrowRight','ArrowRight'],ArrowUp:['ArrowUp','ArrowUp'],ArrowDown:['ArrowDown','ArrowDown']}};
var names=k.split('+'),out=[];for(var i=0;i<names.length;i++){{var n=names[i].trim();if(map[n])out.push({{key:map[n][0],code:map[n][1]}});else if(n.length===1)out.push({{key:n,code:n,text:n}});}}return out;}}
function kd(e,d,type){{var v=e.ownerDocument.defaultView;try{{e.dispatchEvent(new v.KeyboardEvent(type,{{key:d.key,code:d.code||d.key,bubbles:true,cancelable:true}}));}}catch(x){{var ev=e.ownerDocument.createEvent('Event');ev.initEvent(type,true,true);ev.key=d.key;ev.code=d.code||d.key;e.dispatchEvent(ev);}}}}
function setVal(e,val){{var d=e.ownerDocument,v=d.defaultView;
if(e.isContentEditable){{e.textContent=String(val);}}
else{{var proto=(e.tagName||'').toLowerCase()==='textarea'?v.HTMLTextAreaElement&&v.HTMLTextAreaElement.prototype:v.HTMLInputElement&&v.HTMLInputElement.prototype;
var setter=proto&&Object.getOwnPropertyDescriptor(proto,'value')&&Object.getOwnPropertyDescriptor(proto,'value').set;
if(setter)setter.call(e,String(val));else e.value=String(val);}}
e.dispatchEvent(new v.Event('input',{{bubbles:true}}));e.dispatchEvent(new v.Event('change',{{bubbles:true}}));}}
function doAction(ac,ar,e,o){{var v=e.ownerDocument.defaultView;
if(ac==='click'){{e.click();return true;}}
if(ac==='check'||ac==='uncheck'){{var want=(ac==='check');if(!('checked'in e)){{e.click();return true;}}if(e.checked===want){{o.already=true;return true;}}e.click();return true;}}
if(ac==='select'){{if((e.tagName||'').toLowerCase()!=='select')return 'not a <select>';var want=String(ar).toLowerCase(),hit=-1;for(var i=0;i<e.options.length;i++){{var op=e.options[i];if(op.value===String(ar)||(op.text||'').trim().toLowerCase()===want||(op.label||'').toLowerCase()===want){{hit=i;break;}}}}if(hit<0)return 'no-option: '+ar;e.selectedIndex=hit;e.dispatchEvent(new v.Event('input',{{bubbles:true}}));e.dispatchEvent(new v.Event('change',{{bubbles:true}}));o.selected=e.options[hit]?(e.options[hit].text||e.options[hit].value):'';return true;}}
if(ac==='fill'){{e.focus();setVal(e,ar);o.typed=String(ar);return true;}}
if(ac==='type'){{e.focus();var s=String(ar);for(var i=0;i<s.length;i++){{var ch=s[i];kd(e,{{key:ch,code:ch}},'keydown');try{{if(!e.isContentEditable&&'value'in e)e.value+=ch;else if(e.isContentEditable)e.textContent+=ch;}}catch(x){{}}try{{e.dispatchEvent(new v.InputEvent('input',{{bubbles:true,data:ch,inputType:'insertText'}}));}}catch(x){{e.dispatchEvent(new v.Event('input',{{bubbles:true}}));}}kd(e,{{key:ch,code:ch}},'keyup');}}o.typed=s;return true;}}
if(ac==='press'){{e.focus();var defs=keydefs(ar),last=null;for(var i=0;i<defs.length;i++){{last=defs[i];kd(e,last,'keydown');if(last.text){{try{{if(!e.isContentEditable&&'value'in e)e.value+=last.text;else if(e.isContentEditable)e.textContent+=last.text;}}catch(x){{}}try{{e.dispatchEvent(new v.InputEvent('input',{{bubbles:true,data:last.text,inputType:'insertText'}}));}}catch(x){{e.dispatchEvent(new v.Event('input',{{bubbles:true}}));}}}}}}if(last&&last.key==='Enter'){{try{{if(e.form&&e.form.requestSubmit)e.form.requestSubmit();}}catch(x){{}}}}for(var i=defs.length-1;i>=0;i--)kd(e,defs[i],'keyup');o.typed=ar;return true;}}
return 'unknown action';}}
function fin(o,ok,state,msg){{o.status=ok?'ok':'err';o.state=state;if(msg)o.msg=msg;o.elapsed_ms=Date.now()-T0;return o;}}
function report(o){{if(o==null)return null;o.attempts=(o.attempts||0)+1;if(o.status==='ok')return o;if(Date.now()-T0>TMO)return fin(o,false,o.state||'absent',o.msg||'timeout');
var e=resolve(SL);if(!e){{o.state='absent';o.msg='not found';return o;}}
o.desc=desc(e);var r=e.getBoundingClientRect();var bb={{x:r.x,y:r.y,w:r.width,h:r.height}};
if(!vis(e)){{o.state='hidden';o.msg='not visible';o.bbox=bb;return o;}}
if(!enabled(e)){{o.state='disabled';o.msg='not enabled';o.bbox=bb;return o;}}
if(AC==='click'||AC==='check'||AC==='uncheck'){{if(o._bb&&o._bb.x===bb.x&&o._bb.y===bb.y&&o._bb.w===bb.w&&o._bb.h===bb.h){{o._stable=(o._stable||0)+1;}}else{{o._stable=0;}}o._bb=bb;if((o._stable||0)<1){{o.state='stabilizing';o.bbox=bb;return o;}}}}
o.bbox=bb;var res=doAction(AC,AR,e,o);if(res!==true){{if(String(res).indexOf('no-option')===0)return fin(o,false,'no-option',String(res));o.state='error';o.msg=String(res);return o;}}
return fin(o,true,'done',null);}}
window.__NB_ACT=window.__NB_ACT||{{}};var st=window.__NB_ACT;
if(st.id&&st.req!==st.id){{var prev=st[st.req];st[st.req]=report(prev);setTimeout(st.id,IVL);return;}}
st.req=null;var o={{status:'run',action:AC,selector:SL,attempts:0,state:'absent'}};o=report(o);
if(o&&o.status!=='ok'&&(Date.now()-T0<=TMO)){{st.req=SL+'|'+AC;st[st.req]=o;st.id=SL+'|'+AC;return;}}
delete o._bb;delete o._stable;return o;}})()"#,
    js_string(&selector),
    js_string(&act_l),
    js_string(&arg),
    tmo,
    interval_ms
);
crate::agent::browser::eval::eval(js, channel)