// MAR_20250301
#include "mozilla/dom/Document.h"  // Include the Document header
#include <fstream>  // For std::ifstream
#include <sstream>  // For std::stringstream

// Noobscape v2 channel — request dir from env NOOBSCAPE_DIR (default /noobscape),
// one watched request file (inject.js), one response file (inject.out).
#include <cstdlib>   // getenv
#include <cstdio>    // rename
#include <string>
#include <fcntl.h>   // open + O_CREAT|O_EXCL — atomic bind latch
#include <unistd.h>  // write, close
#include "mozilla/dom/BrowsingContext.h"  // BrowsingContext: Id/IsTop/IsContent

static std::string NoobscapeDir() {
  const char* e = getenv("NOOBSCAPE_DIR");
  std::string d = (e && *e) ? std::string(e) : std::string("/noobscape");
  if (!d.empty() && d.back() == '/') d.pop_back();
  return d;
}
static std::string NoobscapeReqPath() { return NoobscapeDir() + "/inject.js"; }
static std::string NoobscapeOutPath() { return NoobscapeDir() + "/inject.out"; }
static std::string NoobscapeBindIdPath()  { return NoobscapeDir() + "/bind.id"; }
static std::string NoobscapeBindUrlPath() { return NoobscapeDir() + "/bind.url"; }

// Read a small channel file, trimming trailing whitespace/newlines.
static std::string NoobscapeReadFile(const std::string& p) {
  std::ifstream f(p.c_str(), std::ios::binary);
  if (!f) return std::string();
  std::stringstream ss; ss << f.rdbuf();
  std::string s = ss.str();
  while (!s.empty() && (s.back()=='\n' || s.back()=='\r' || s.back()==' ' || s.back()=='\t')) s.pop_back();
  return s;
}

// Atomically create bind.id with our top browsing-context id. O_EXCL means the
// first writer wins; any later caller takes the already-bound path instead.
static bool NoobscapeLatch(uint64_t id) {
  int fd = ::open(NoobscapeBindIdPath().c_str(), O_CREAT | O_EXCL | O_WRONLY, 0644);
  if (fd < 0) return false;
  std::string s = std::to_string(id);
  ssize_t w = ::write(fd, s.data(), s.size());
  (void)w;
  ::close(fd);
  return true;
}

// v2 request dedup — per process, i.e. per Noobscape browser.
static std::string gNoobscapeLastId;

static void NoobscapeWriteOut(const std::string& id, bool ok,
                              const std::string& payload) {
  std::string outPath = NoobscapeOutPath();
  std::string tmp = outPath + ".tmp";
  { std::ofstream f(tmp.c_str(), std::ios::binary | std::ios::trunc);
    if (!f) return;
    f << id << "\n" << (ok ? "OK" : "ERR") << "\n" << payload; }
  rename(tmp.c_str(), outPath.c_str());
}

// Forward declarations
class FileWatcher;
static void NoobscapeInject(nsDocShell* self, nsIFile* file);
static void NoobscapeStartWatcher(nsDocShell* self);

// Global map to keep track of watchers per docshell
static std::map<nsDocShell*, RefPtr<FileWatcher>> gFileWatchers;
static mozilla::Mutex gFileWatcherMutex("FileWatcherMutex");

class FileWatcher final : public nsIObserver {
public:
  NS_DECL_ISUPPORTS

  explicit FileWatcher(nsDocShell* aDocShell)
      : mDocShell(new nsMainThreadPtrHolder<nsDocShell>("FileWatcher::mDocShell", aDocShell))
      , mRunning(false) {}

  // Implement the nsIObserver interface
  NS_IMETHOD Observe(nsISupports* aSubject, const char* aTopic,
                     const char16_t* aData) override {
    if (strcmp(aTopic, "dom-window-destroyed")) return NS_OK;
    // The teardown below drops the map's ref and the observer service's ref,
    // either of which could be the last one — keep ourselves alive through it.
    RefPtr<FileWatcher> kungFuDeathGrip(this);
    // Our docshell is dead when the outer window being destroyed is ours (or
    // when the docshell has already lost its window). Tear down: drop the map
    // entry, stop the poll thread, unregister. Without this, every navigation
    // strands one watcher thread on the dead page — "document-unload" was
    // never notified (not a real topic) and the weak-ref registration failed
    // besides (no nsISupportsWeakReference on this class).
    nsCOMPtr<nsPIDOMWindowOuter> destroyed = do_QueryInterface(aSubject);
    nsCOMPtr<nsPIDOMWindowOuter> mine =
        mDocShell.get() ? mDocShell.get()->GetWindow() : nullptr;
    if (!mine || (destroyed && destroyed == mine)) {
      {
        mozilla::MutexAutoLock lock(gFileWatcherMutex);
        gFileWatchers.erase(mDocShell);
      }
      Stop();
    }
    return NS_OK;
  }

                     nsresult Start() {
                       mozilla::MutexAutoLock lock(mMutex);
                       if (mRunning) return NS_OK;

                       mRunning = true;
                       mThread = PR_CreateThread(PR_USER_THREAD, ThreadFunc, this, PR_PRIORITY_NORMAL,
                                                 PR_GLOBAL_THREAD, PR_JOINABLE_THREAD, 0);
                       if (!mThread) {
                         mRunning = false;
                         return NS_ERROR_FAILURE;
                       }

                       // Register for outer-window destruction — the docshell-death
                       // signal that drives cleanup. Strong ref (aOwnsWeak=false):
                       // this class has no nsISupportsWeakReference, so a weak
                       // registration is refused and the observer never fires.
                       nsCOMPtr<nsIObserverService> obs = mozilla::services::GetObserverService();
                       if (obs) {
                         obs->AddObserver(this, "dom-window-destroyed", false);
                       }

                       return NS_OK;
                     }

                     void Stop() {
                       PRThread* toJoin = nullptr;
                       {
                         mozilla::MutexAutoLock lock(mMutex);
                         if (!mRunning) return;
                         mRunning = false;
                         toJoin = mThread;
                         mThread = nullptr;
                       }

                       // Join OUTSIDE the lock: the poll loop takes mMutex at the
                       // top of every tick, so holding it across the join deadlocks.
                       if (toJoin) {
                         PR_Interrupt(toJoin);
                         PR_JoinThread(toJoin);
                       }

                       // Unregister observer
                       nsCOMPtr<nsIObserverService> obs = mozilla::services::GetObserverService();
                       if (obs) {
                         obs->RemoveObserver(this, "dom-window-destroyed");
                       }
                     }

private:
  ~FileWatcher() {
    Stop();
  }

  static void PR_CALLBACK ThreadFunc(void* arg) {
    auto* self = static_cast<FileWatcher*>(arg);
    self->WatchFile();
  }

  void WatchFile() {
    static const PRIntervalTime kCheckInterval = PR_MillisecondsToInterval(1000); // Check every second
    PRTime lastModifiedTime = 0;

    while (true) {
      {
        mozilla::MutexAutoLock lock(mMutex);
        if (!mRunning) break;
      }

      // Check if file has been modified
      nsCOMPtr<nsIFile> file;
      nsCString nativePath(NoobscapeReqPath().c_str());
      nsresult rv = NS_NewNativeLocalFile(nativePath, true, getter_AddRefs(file));

      if (NS_SUCCEEDED(rv)) {
        bool exists = false;
        rv = file->Exists(&exists);

        if (NS_SUCCEEDED(rv) && exists) {
          PRTime modifiedTime = 0;
          file->GetLastModifiedTime(&modifiedTime);

          if (modifiedTime > lastModifiedTime) {
            lastModifiedTime = modifiedTime;

            // Schedule script injection on the main thread
            nsCOMPtr<nsIRunnable> runnable = NS_NewRunnableFunction(
                "nsDocShell::InjectScriptIntoDOM",
                [handle = mDocShell, file]() {
                    // This lambda runs on the Main Thread.
                    // It is now safe to dereference the handle.
                    if (handle) {
                        NoobscapeInject(handle.get(), file);
                    }
                });

            NS_DispatchToMainThread(runnable);
          }
        }
      }

      // Sleep for the check interval
      PR_Sleep(kCheckInterval);
    }
  }

  nsMainThreadPtrHandle<nsDocShell> mDocShell;
  mozilla::Mutex mMutex{"FileWatcherMutex"};
  bool mRunning;
  PRThread* mThread;
};

// Implementation of nsISupports for FileWatcher
NS_IMPL_ISUPPORTS(FileWatcher, nsIObserver)

// ---------------------------------------------------------------------------
// Noobscape v2 free-function mechanism (no nsDocShell.h changes required:
// GetDocument() and GetWindow() are both public on nsDocShell).
// ---------------------------------------------------------------------------

#include "nsIAppStartup.h"
#include "mozilla/Services.h"

static void NoobscapeForceQuit() {
  nsCOMPtr<nsIAppStartup> appStartup = do_GetService("@mozilla.org/toolkit/app-startup;1");
  if (appStartup) {
    printf("Noobscape: force quitting...\n");
    bool ignoredRetVal;
    appStartup->Quit(nsIAppStartup::eForceQuit, 0, &ignoredRetVal);
  } else {
    printf("Noobscape: failed to get nsIAppStartup service.\n");
  }
}

#include "nsIScriptSecurityManager.h"  // nsIScriptSecurityManager
#include "nsContentUtils.h"            // nsContentUtils
#include "jsapi.h"
#include "js/CompilationAndEvaluation.h"
#include "js/SourceText.h"
#include "js/JSON.h"                   // JS_Stringify
#include "js/Conversions.h"           // JS::ToString
#include "mozilla/dom/ScriptSettings.h"  // AutoEntryScript
#include "nsIGlobalObject.h"

// JSONWriteCallback: bool(const char16_t*, uint32_t, void*).
static bool NoobscapeJSONWrite(const char16_t* buf, uint32_t len, void* data) {
  static_cast<nsAutoString*>(data)->Append(buf, len);
  return true;
}

static void NoobscapeInject(nsDocShell* self, nsIFile* file) {
  if (!self || !file) return;

  // Read raw request bytes.
  nsCString nativePath = file->NativePath();
  std::ifstream infile(nativePath.get(), std::ios::binary);
  if (!infile) return;
  std::stringstream ss; ss << infile.rdbuf();
  std::string raw = ss.str();

  // v2 sentinel: first line exactly "//NOOBSCAPE"; line 2 = id; rest = JS.
  bool v2 = false; bool chromeMode = false; std::string id, jsUtf8;
  {
    size_t nl1 = raw.find('\n');
    std::string l1 = (nl1 == std::string::npos) ? raw : raw.substr(0, nl1);
    while (!l1.empty() && l1.back() == '\r') l1.pop_back();
    if ((l1 == "//NOOBSCAPE" || l1 == "//NOOBSCAPE-CHROME") && nl1 != std::string::npos) {
      v2 = true;
      chromeMode = (l1 == "//NOOBSCAPE-CHROME");
      size_t nl2 = raw.find('\n', nl1 + 1);
      if (nl2 == std::string::npos) { id = raw.substr(nl1 + 1); }
      else { id = raw.substr(nl1 + 1, nl2 - (nl1 + 1)); jsUtf8 = raw.substr(nl2 + 1); }
      while (!id.empty() && (id.back() == '\r' || id.back() == '\n')) id.pop_back();
    }
  }

  nsAutoString scriptContent;
  scriptContent.Assign(NS_ConvertUTF8toUTF16((v2 ? jsUtf8 : raw).c_str()));
  scriptContent.Trim(" \t\r\n");

  if (scriptContent.EqualsLiteral("QUIT")) {
    if (v2) NoobscapeWriteOut(id, true, "null");
    ::remove(NoobscapeBindIdPath().c_str());   // let the next session re-latch
    NoobscapeForceQuit();
    return;
  }

  // ---- Noobscape v2 bind gate --------------------------------------------
  // Firefox runs many docshells (the real tab, the page-thumbnail service,
  // extension background pages), and every one watches the same request file.
  // Ungated they race to answer and the last writer wins. We bind to exactly
  // one tab: the FIRST top-level *content* docshell that loads the launch URL
  // latches its BROWSER id (the tab's stable identity) into bind.id, and from
  // then on ONLY that tab answers. The browser id -- unlike the browsing-
  // context id -- survives cross-process navigations, which REPLACE the
  // browsing context with a fresh id; the file makes the latch visible across
  // the parent/content process split that the per-process dedup static alone
  // cannot bridge.
  if (v2) {
    mozilla::dom::Document* gdoc = self->GetDocument();
    mozilla::dom::BrowsingContext* bc = gdoc ? gdoc->GetBrowsingContext() : nullptr;
    if (!bc) return;  // no browsing context yet — unanswerable; stay silent
    if (chromeMode) {
      // CHROME-targeted request: answered by a top-level CHROME (non-content)
      // docshell — a parent-process chrome window. Content docshells stay
      // silent (they lack chrome WebIDL and cannot reach WindowGlobalParent).
      // Among the parent's chrome docshells the per-process dedup below elects
      // one answerer; the payload targets the bound tab by id via
      // BrowsingContext.getCurrentTopByBrowserId(bind.id)
      //   .currentWindowGlobal.drawSnapshot(...).
      if (!(bc->IsTop() && !bc->IsContent())) return;  // silent
    } else {
    // Browser id, not browsing-context id: a cross-process navigation
    // REPLACES the browsing context (fresh id), and the pre-navigation page
    // lingers in the bfcache with a live watcher. Gate on the tab's stable
    // browser id and silence any context that no longer fronts the tab.
    if (bc->IsDiscarded() || bc->IsInBFCache()) return;  // silent
    uint64_t myId = bc->BrowserId();
    std::string boundStr = NoobscapeReadFile(NoobscapeBindIdPath());
    if (!boundStr.empty()) {
      // Bound: only the current top docshell of the latched tab may answer.
      uint64_t boundId = strtoull(boundStr.c_str(), nullptr, 10);
      if (!(bc->IsTop() && myId == boundId)) return;  // silent
    } else {
      // Unbound: latch iff I am a top-level content docshell at the launch URL.
      if (!(bc->IsTop() && bc->IsContent())) return;  // silent
      std::string launch = NoobscapeReadFile(NoobscapeBindUrlPath());
      if (!launch.empty()) {
        std::string mine;
        nsCOMPtr<nsIURI> u = gdoc->GetDocumentURI();
        if (u) { nsAutoCString spec; u->GetSpec(spec); mine.assign(spec.get()); }
        auto strip = [](std::string v) { while (!v.empty() && v.back()=='/') v.pop_back(); return v; };
        if (strip(mine) != strip(launch)) return;  // not our tab yet — silent
      }
      if (!NoobscapeLatch(myId)) {
        // Lost the create race (only one docshell loads the launch URL, so
        // this should not happen); proceed only if the winner is in fact us.
        std::string b2 = NoobscapeReadFile(NoobscapeBindIdPath());
        if (strtoull(b2.c_str(), nullptr, 10) != myId) return;
      }
    }
    }
  }

  // Dedup AFTER the gate: only the docshell that will actually answer consumes
  // the id, so a subframe sharing this process can never eat a request meant
  // for the top document.
  if (v2) {
    if (id == gNoobscapeLastId) return;   // run each id at most once
    gNoobscapeLastId = id;
  }

  nsIScriptSecurityManager* ssm = nsContentUtils::GetSecurityManager();
  if (!ssm) { if (v2) NoobscapeWriteOut(id, false, "no security manager"); return; }
  nsCOMPtr<nsIPrincipal> systemPrincipal;
  ssm->GetSystemPrincipal(getter_AddRefs(systemPrincipal));

  mozilla::dom::Document* doc = self->GetDocument();
  if (!doc) { if (v2) NoobscapeWriteOut(id, false, "no document"); return; }
  nsCOMPtr<nsPIDOMWindowOuter> outerWindow = self->GetWindow();
  if (!outerWindow) { if (v2) NoobscapeWriteOut(id, false, "no window"); return; }
  nsCOMPtr<nsPIDOMWindowInner> innerWindow = outerWindow->GetCurrentInnerWindow();
  if (!innerWindow) { if (v2) NoobscapeWriteOut(id, false, "no inner window"); return; }

  // AutoEntryScript, not AutoJSAPI: pushes the entry-script state that
  // script-initiated navigation (location assignment, link clicks) consults
  // when committing a load; under bare AutoJSAPI the JS runs and returns but
  // the navigation silently never commits.
  nsIGlobalObject* aesGlobal = innerWindow->AsGlobal();
  if (!aesGlobal || !aesGlobal->HasJSGlobal()) { if (v2) NoobscapeWriteOut(id, false, "no JS global"); return; }
  mozilla::dom::AutoEntryScript aes(aesGlobal, "Noobscape", true);
  JSContext* cx = aes.cx();
  JS::RootedValue rval(cx);
  JS::CompileOptions options(cx);
  options.setFileAndLine("injected-script.js", 1);

  JS::SourceText<char16_t> source;
  if (!source.init(cx, scriptContent.get(), scriptContent.Length(),
                   JS::SourceOwnership::Borrowed)) {
    if (v2) NoobscapeWriteOut(id, false, "source init failed");
    return;
  }

  bool okEval = JS::Evaluate(cx, options, source, &rval);
  if (!v2) { if (!okEval) printf("Noobscape: script execution failed.\n"); return; }  // legacy

  if (!okEval) {
    std::string msg = "script threw";
    JS::RootedValue exc(cx);
    if (JS_GetPendingException(cx, &exc)) {
      JS_ClearPendingException(cx);
      JS::RootedString es(cx, JS::ToString(cx, exc));
      if (es) { JS::UniqueChars ec = JS_EncodeStringToUTF8(cx, es);
                if (ec) msg = ec.get(); }
    }
    NoobscapeWriteOut(id, false, msg);
    return;
  }

  if (rval.isUndefined()) { NoobscapeWriteOut(id, true, "null"); return; }
  nsAutoString jsonOut;
  JS::RootedValue space(cx);
  if (JS_Stringify(cx, &rval, nullptr, space, NoobscapeJSONWrite, &jsonOut)) {
    NoobscapeWriteOut(id, true, NS_ConvertUTF16toUTF8(jsonOut).get());
    return;
  }
  // Not JSON-serialisable (e.g. a DOM node): JSON string of String(rval).
  JS_ClearPendingException(cx);
  JS::RootedString s(cx, JS::ToString(cx, rval));
  if (s) {
    JS::RootedValue sv(cx, JS::StringValue(s));
    nsAutoString sout;
    if (JS_Stringify(cx, &sv, nullptr, space, NoobscapeJSONWrite, &sout)) {
      NoobscapeWriteOut(id, true, NS_ConvertUTF16toUTF8(sout).get());
      return;
    }
  }
  NoobscapeWriteOut(id, true, "null");
}

static void NoobscapeStartWatcher(nsDocShell* self) {
  if (!self) return;
  mozilla::dom::Document* doc = self->GetDocument();
  if (!doc) { printf("Noobscape: no document in StartWatcher.\n"); return; }

  nsCOMPtr<nsIFile> file;
  nsCString nativePath(NoobscapeReqPath().c_str());
  nsresult rv = NS_NewNativeLocalFile(nativePath, true, getter_AddRefs(file));
  if (NS_FAILED(rv)) { printf("Noobscape: NS_NewNativeLocalFile failed: %X\n", static_cast<uint32_t>(rv)); return; }

  // Start the watcher UNCONDITIONALLY (v2): the request file may not exist yet
  // — the driver writes it after launch. WatchFile no-ops while it is absent.
  mozilla::MutexAutoLock lock(gFileWatcherMutex);
  if (gFileWatchers.find(self) != gFileWatchers.end()) return;  // already watching

  RefPtr<FileWatcher> watcher = new FileWatcher(self);
  nsresult startResult = watcher->Start();
  if (NS_SUCCEEDED(startResult)) {
    gFileWatchers[self] = watcher;
    printf("Noobscape: started watcher for %s\n", NoobscapeReqPath().c_str());
    // Inject immediately only if a request is already present.
    bool exists = false;
    if (NS_SUCCEEDED(file->Exists(&exists)) && exists) { NoobscapeInject(self, file); }
  } else {
    printf("Noobscape: failed to start watcher.\n");
  }
}
