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
                       if (!strcmp(aTopic, "document-unload")) {
                         // Since we can't directly query for nsIDocument (it's not scriptable),
                         // we'll compare URIs instead
                         nsCOMPtr<mozilla::dom::Document> currentDoc = mDocShell->GetDocument();
                         if (!currentDoc) {
                           return NS_OK;
                         }

                         // Try to get the document from the subject
                         RefPtr<mozilla::dom::Document> unloadedDoc;
                         nsCOMPtr<nsINode> node = do_QueryInterface(aSubject);
                         if (node) {
                           unloadedDoc = node->OwnerDoc();
                         }

                         // If we can't get the document directly, the event isn't relevant to us
                         if (!unloadedDoc) {
                           return NS_OK;
                         }

                         // Compare the URIs
                         nsCOMPtr<nsIURI> currentURI = currentDoc->GetDocumentURI();
                         nsCOMPtr<nsIURI> unloadedURI = unloadedDoc->GetDocumentURI();

                         bool equal = false;
                         if (currentURI && unloadedURI) {
                           currentURI->Equals(unloadedURI, &equal);
                         }

                         if (equal) {
                           // Clean up the watcher when document unloads
                           Stop();

                           // Remove from global map
                           mozilla::MutexAutoLock lock(gFileWatcherMutex);
                           gFileWatchers.erase(mDocShell);
                         }
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

                       // Register observer for document unload
                       nsCOMPtr<nsIObserverService> obs = mozilla::services::GetObserverService();
                       if (obs) {
                         obs->AddObserver(this, "document-unload", true);
                       }

                       return NS_OK;
                     }

                     void Stop() {
                       mozilla::MutexAutoLock lock(mMutex);
                       if (!mRunning) return;

                       mRunning = false;

                       // Wake up the thread if it's sleeping
                       PR_Interrupt(mThread);

                       // Wait for thread to finish
                       PR_JoinThread(mThread);
                       mThread = nullptr;

                       // Unregister observer
                       nsCOMPtr<nsIObserverService> obs = mozilla::services::GetObserverService();
                       if (obs) {
                         obs->RemoveObserver(this, "document-unload");
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
  bool v2 = false; std::string id, jsUtf8;
  {
    size_t nl1 = raw.find('\n');
    std::string l1 = (nl1 == std::string::npos) ? raw : raw.substr(0, nl1);
    while (!l1.empty() && l1.back() == '\r') l1.pop_back();
    if (l1 == "//NOOBSCAPE" && nl1 != std::string::npos) {
      v2 = true;
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
  // latches its top browsing-context id into bind.id, and from then on ONLY
  // that id answers. The id survives goto() navigations (the document is
  // replaced, the browsing context is not), and the file makes the latch
  // visible across the parent/content process split that the per-process
  // dedup static alone cannot bridge.
  if (v2) {
    mozilla::dom::Document* gdoc = self->GetDocument();
    mozilla::dom::BrowsingContext* bc = gdoc ? gdoc->GetBrowsingContext() : nullptr;
    if (!bc) return;  // no browsing context yet — unanswerable; stay silent
    uint64_t myId = bc->Id();
    std::string boundStr = NoobscapeReadFile(NoobscapeBindIdPath());
    if (!boundStr.empty()) {
      // Bound: only the top-level docshell whose id matches may answer.
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

  // Dedup AFTER the gate: only the docshell that will actually answer consumes
  // the id, so a subframe sharing this process can never eat a request meant
  // for the top document.
  if (v2) {
    if (id == gNoobscapeLastId) return;   // run each id at most once
    gNoobscapeLastId = id;
  }

  // v2 navigation directive: a chrome-principal script cannot drive content
  // navigation via location.href / location.assign / pushState — those do a
  // same-origin / subject-principal check that the SYSTEM principal (which the
  // injected script carries) fails, so the load silently vetoes. The driver
  // sends "NOOBSCAPE_NAV <url>" instead and we perform a real top-level
  // docshell load with a system triggering principal — the same path the URL
  // bar uses. The bind gate above means only the bound tab navigates.
  if (v2 && StringBeginsWith(scriptContent, u"NOOBSCAPE_NAV "_ns)) {
    nsAutoString navUrl(Substring(scriptContent, 14));
    navUrl.Trim(" \t\r\n");
    mozilla::dom::LoadURIOptions navOpts;
    navOpts.mTriggeringPrincipal = nsContentUtils::GetSystemPrincipal();
    nsresult navRv = self->FixupAndLoadURIString(navUrl, navOpts);
    NoobscapeWriteOut(id, NS_SUCCEEDED(navRv), NS_SUCCEEDED(navRv) ? "null" : "load failed");
    return;
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

  // v2: AutoEntryScript (not AutoJSAPI) establishes the entry-script settings
  // a script-initiated navigation (location.href=, location.assign, goto) needs
  // to resolve its source browsing context; AutoJSAPI leaves that unset, so such
  // navigations silently no-op. Entering the global's realm is handled by aes.
  nsCOMPtr<nsIGlobalObject> global = do_QueryInterface(innerWindow);
  if (!global) { if (v2) NoobscapeWriteOut(id, false, "no global object"); return; }
  AutoEntryScript aes(global, "Noobscape", true);
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
  if (NS_FAILED(rv)) { printf("Noobscape: NS_NewNativeLocalFile failed: %X\n", rv); return; }

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
