// MAR_20250301
#include "mozilla/dom/Document.h"  // Include the Document header
#include <fstream>  // For std::ifstream
#include <sstream>  // For std::stringstream

// Noobscape v2 channel — request dir from env NOOBSCAPE_DIR (default /noobscape),
// one watched request file (inject.js), one response file (inject.out).
#include <cstdlib>   // getenv
#include <cstdio>    // rename
#include <string>

static std::string NoobscapeDir() {
  const char* e = getenv("NOOBSCAPE_DIR");
  std::string d = (e && *e) ? std::string(e) : std::string("/noobscape");
  if (!d.empty() && d.back() == '/') d.pop_back();
  return d;
}
static std::string NoobscapeReqPath() { return NoobscapeDir() + "/inject.js"; }
static std::string NoobscapeOutPath() { return NoobscapeDir() + "/inject.out"; }

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

  if (v2) {
    if (id == gNoobscapeLastId) return;   // run each id at most once
    gNoobscapeLastId = id;
  }

  nsAutoString scriptContent;
  scriptContent.Assign(NS_ConvertUTF8toUTF16((v2 ? jsUtf8 : raw).c_str()));
  scriptContent.Trim(" \t\r\n");

  if (scriptContent.EqualsLiteral("QUIT")) {
    if (v2) NoobscapeWriteOut(id, true, "null");
    NoobscapeForceQuit();
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

  AutoJSAPI jsapi;
  if (!jsapi.Init(innerWindow)) { if (v2) NoobscapeWriteOut(id, false, "jsapi init failed"); return; }
  JSContext* cx = jsapi.cx();
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
