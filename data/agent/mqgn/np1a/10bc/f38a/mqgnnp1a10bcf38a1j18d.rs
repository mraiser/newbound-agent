// Auto-waiting browser interaction: poll (in ONE page-side eval) until an
// element matching `selector` is attached AND visible, then dispatch `action`.
// The entire wait runs inside the page: the eval re-arms itself via
// setTimeout(<id>, 0) — the C++ writes the response file synchronously and the
// request is consumed on arrival, so the armed timeout re-runs this same source
// later in the same page, against the CURRENT document (the dedup latch does
// not block re-execution, only a response rewrite). One file round-trip total,
// instead of N. Requires an eval channel at least ~300 ms older than
// timeout_ms to ever answer; the caller's eval timeout is padded accordingly.
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
// The page-side poll interval the script will re-arm with; the eval channel
// must outlive at least one of those cycles to ever see a result.
let interval_ms: i64 = 250;
let channel = tmo * 3 + 15000;

let js = format!(
    r#"(function(){{
var SL={}, AC={}, AR={}, D={}, TMO={}, IVL={};
function txt(n){{return (n.textContent||'').replace(/\s+/g,' ').trim();}}
function vis(e){{if(!e||!e.isConnected)return false;var d=e.ownerDocument||document;var r=e.getBoundingClientRect();if(r.width<1||r.height<1)return false;var s=d.defaultView.getComputedStyle(e);return s.display!=='none'&&s.visibility!=='hidden'&&parseFloat(s.opacity||'1')>0;}}
function walk(c,fn,depth){{fn(c);if(depth>24)return;var fr;try{{fr=c.querySelectorAll('iframe,frame');}}catch(x){{return;}}for(var i=0;i<fr.length;i++){{var fd=null;try{{fd=fr[i].contentDocument;}}catch(x){{fd=null;}}if(fd)walk(fd,fn,depth+1);}}}}
function one(doc,base,spec){{var kind=spec.k,val=spec.v,sub=spec.s,visf=spec.n,nth=spec.i||0;if(kind==='css'){{var out=[];walk(doc,function(c){{try{{var r=c.querySelectorAll(base);for(var j=0;j<r.length;j++)out.push([r[j],c]);}}catch(x){{}}}});return [out,nth];}}
var list=[];walk(doc,function(c){{var a;try{{a=c.querySelectorAll(kind==='xp'?'body':'body,body *');}}catch(x){{a=[];}}for(var j=0;j<a.length;j++){{var e=a[j];if(kind==='text'&&txt(e).indexOf(val)>=0)list.push([e,c]);if(kind==='has'&&txt(e).toLowerCase().indexOf(val.toLowerCase())>=0)list.push([e,c]);if(kind==='role'&&((e.getAttribute&&e.getAttribute('role')||'')===val||(e.tagName||'').toLowerCase()===val.toLowerCase()))list.push([e,c]);if(kind==='xp'){{try{{if(c.evaluate(val,c,null,9,null).singleNodeValue===e)list.push([e,c]);}}catch(x){{}}}}}}}});if(sub)list=list.filter(function(p){{return txt(p[0]).indexOf(sub)>=0;}});if(visf)list=list.filter(function(p){{return vis(p[0]);}});return [list,nth];}}
function parse(s){{var steps=[],re=/(?:^|\s*)>>\s*/g,m,last=0;while((m=re.exec(s))){{steps.push(s.slice(last,m.index));last=m.index+m[0].length;}}steps.push(s.slice(last));steps=steps.filter(function(x){{return x.trim().length>0;}});var out=[];for(var i=0;i<steps.length;i++){{var st=steps[i].trim(),nth=0,nm;nm=st.match(/^(.*):nth\((-?\d+)\)$/);if(nm){{st=nm[1];nth=parseInt(nm[2],10);}}var visf=false;if(/:visible$/.test(st)){{visf=true;st=st.replace(/:visible$/,'');}}var mm;if((mm=st.match(/^text=(.*)$/)))out.push({{k:'text',v:mm[1].trim(),i:nth,n:visf}});else if((mm=st.match(/^role=(.*)$/)))out.push({{k:'role',v:mm[1].trim(),i:nth,n:visf}});else if((mm=st.match(/^xpath=(.*)$/)))out.push({{k:'xp',v:mm[1].trim(),i:nth,n:visf}});else if((mm=st.match(/^:has-text\((.*)\)$/)))out.push({{k:'has',v:mm[1].trim(),i:nth,n:visf}});else if((mm=st.match(/^(.*?):has-text\((.*)\)$/)))out.push({{k:'css',v:mm[1].trim(),s:mm[2],i:nth,n:visf}});else out.push({{k:'css',v:st,i:nth,n:visf}});}}
return out;}}
function resolve(sl){{var steps=parse(sl),cs=[document],cn=[null];for(var i=0;i<steps.length;i++){{var nx=[],nd=[],spec=steps[i];for(var j=0;j<cs.length;j++){{var r=one(cs[j],spec.v,spec),list=r[0],nth=r[1];if(list.length>0){{var idx=nth<0?list.length+nth:nth;if(idx<0||idx>=list.length)idx=0;nx.push(list[idx][0]);nd.push(list[idx][1]);}}}}if(nx.length===0)return null;cs=nx;cn=nd;}}
var e=cs[cs.length-1];if(e&&e.getRootNode&&e.getRootNode()instanceof ShadowRoot)return[e,cn[cn.length-1]||e.getRootNode()];return[e,cn[cn.length-1]||document];}}
function focusable(e){{if(!e)return false;if(e.disabled)return false;var t=(e.tagName||'').toLowerCase();if(/^(input|textarea|select|button)$/.test(t))return true;if(e.isContentEditable)return true;if(e.tabIndex>=0)return true;var ae=e.ownerDocument.activeElement;return ae===e;}}
function keydefs(k){{k=String(k);if(k.length===1)return[{{key:k,code:'',text:k}}];var map={{Enter:{{key:'Enter',code:'Enter'}},Tab:{{key:'Tab',code:'Tab'}},Escape:{{key:'Escape',code:'Escape'}},Backspace:{{key:'Backspace',code:'Backspace'}},Delete:{{key:'Delete',code:'Delete'}},Home:{{key:'Home',code:'Home'}},End:{{key:'End',code:'End'}},PageUp:{{key:'PageUp',code:'PageUp'}},PageDown:{{key:'PageDown',code:'PageDown'}},ArrowLeft:{{key:'ArrowLeft',code:'ArrowLeft'}},ArrowRight:{{key:'ArrowRight',code:'ArrowRight'}},ArrowUp:{{key:'ArrowUp',code:'ArrowUp'}},ArrowDown:{{key:'ArrowDown',code:'ArrowDown'}}}};var names=k.split('+');var out=[];for(var i=0;i<names.length;i++){{var n=names[i].trim();var d=map[n];if(d)out.push({{key:d.key,code:d.code}});else if(n.length===1)out.push({{key:n,code:'',text:n}});}}
return out;}}
function dispatchKey(e,d,type){{var v=e.ownerDocument.defaultView;try{{e.dispatchEvent(new v.KeyboardEvent(type,{{key:d.key,code:d.code||d.key,bubbles:true,cancelable:true}}));}}catch(x){{var ev=e.ownerDocument.createEvent('Event');ev.initEvent(type,true,true);ev.key=d.key;ev.code=d.code||d.key;e.dispatchEvent(ev);}}}}
function doAction(ac,ar,e){{var v=e.ownerDocument.defaultView,d=e.ownerDocument;if(ac==='click'){{e.click();return true;}}
if(ac==='check'||ac==='uncheck'){{var want=(ac==='check');if(!('checked'in e)){{e.click();return true;}}if(e.checked!==want)e.click();return true;}}
if(ac==='select'){{if((e.tagName||'').toLowerCase()!=='select')return 'not a <select>';var want=String(ar).toLowerCase(),hit=-1;for(var i=0;i<e.options.length;i++){{var o=e.options[i];if(o.value===String(ar)||(o.text||'').toLowerCase()===want){{hit=i;break;}}}}if(hit<0)return 'no matching option: '+ar;e.selectedIndex=hit;e.dispatchEvent(new v.Event('input',{{bubbles:true}}));e.dispatchEvent(new v.Event('change',{{bubbles:true}}));return true;}}
if(ac==='fill'){{e.focus();if(!e.isContentEditable)e.value=String(ar);else e.textContent=String(ar);e.dispatchEvent(new v.Event('input',{{bubbles:true}}));e.dispatchEvent(new v.Event('change',{{bubbles:true}}));return true;}}
if(ac==='type'){{e.focus();var s=String(ar);for(var i=0;i<s.length;i++){{var ch=s[i];dispatchKey(e,{{key:ch,code:ch}},'keydown');try{{if(!e.isContentEditable&&'value'in e)e.value+=ch;else if(e.isContentEditable)e.textContent+=ch;}}catch(x){{}}e.dispatchEvent(new v.InputEvent('input',{{bubbles:true,data:ch,inputType:'insertText'}}));dispatchKey(e,{{key:ch,code:ch}},'keyup');}}return true;}}
if(ac==='press'){{e.focus();var defs=keydefs(ar),last=null;for(var i=0;i<defs.length;i++){{last=defs[i];dispatchKey(e,last,'keydown');if(last.text){{try{{if(!e.isContentEditable&&'value'in e)e.value+=last.text;else if(e.isContentEditable)e.textContent+=last.text;}}catch(x){{}}e.dispatchEvent(new v.InputEvent('input',{{bubbles:true,data:last.text,inputType:'insertText'}}));}}}}
if(last&&last.key==='Enter'){{try{{if(e.form&&e.form.requestSubmit)e.form.requestSubmit();}}catch(x){{}}}}
for(var i=defs.length-1;i>=0;i--)dispatchKey(e,defs[i],'keyup');return true;}}
return 'unknown action';}}
function report(o){{if(o==null)return null;o.retries=(o.retries||0)+1;if(o.ok||o.done)return o;if(Date.now()>D+TMO){{o.done=true;o.err='timeout';return o;}}var r=resolve(SL);if(!r||!r[0]){{o.err='not found';return o;}}
var e=r[0],root=r[1];var rr=e.getBoundingClientRect();o.bbox={{x:rr.x,y:rr.y,w:rr.width,h:rr.height}};
if(!vis(e)){{o.err='not visible';return o;}}
if(AC==='click'&&!focusable(e)){{o.err='not enabled';return o;}}
var res=doAction(AC,AR,e);if(res!==true){{o.err=String(res);return o;}}
o.ok=true;o.err=null;return o;}}
window.__NB_ACT=window.__NB_ACT||{{}};var st=window.__NB_ACT;
if(st.id&&st.req!==st.id){{var prev=st[st.req];st[st.req]=report(prev);setTimeout(st.id,0);return;}}
st.req=null;var o={{sel:SL,action:AC,ok:false,err:null,retries:0}};o=report(o);if(o&&!o.ok&&!o.done){{st.req=SL+'|'+AC;st[st.req]=o;st.id=SL+'|'+AC;return;}}
return o;}})()"#,
    js_string(&selector),
    js_string(&act_l),
    js_string(&arg),
    flowlang::flowlang::system::time::time(),
    tmo,
    interval_ms
);
crate::agent::browser::eval::eval(js, channel)