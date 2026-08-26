// Navigate the bound Noobscape session to `url`.
//
// In-tab script navigation is not possible in this browser build, and the
// reason is structural rather than a bug to patch around:
//  - Injected scripts run with the SYSTEM principal (that is what gives eval
//    its chrome powers), so DOM navigation (location.href / assign / pushState)
//    fails its same-origin / subject-principal check and is silently vetoed.
//  - A chrome-context docshell load from the CONTENT process (where the tab
//    lives under Fission) returns NS_OK but never commits: a top-level load
//    must originate in the owning flow, which an injected watcher is not part
//    of. Verified empirically (parent=0 content=1 isTop=1, rv=NS_OK, no load).
//
// The one primitive that reliably drives this browser is the command-line
// launch open() already uses. goto() therefore REBINDS: it closes the current
// session and relaunches at `url`, and the bind latch attaches to the fresh
// tab. The default profile persists, so cookies and localStorage survive; only
// in-page JS state resets — exactly what a real navigation does. Returns
// open()'s {status, pid, url}.
let _ = crate::agent::browser::close::close();
// close() TERMs the recorded pid; let the profile's single-instance lock
// release before relaunch so firefox does not refuse it as already running.
std::thread::sleep(std::time::Duration::from_millis(800));
crate::agent::browser::open::open(url)