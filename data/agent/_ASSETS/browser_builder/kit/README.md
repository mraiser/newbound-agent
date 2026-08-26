# Noobscape v2 build workspace

This directory builds **Noobscape** — a Firefox 128 ESR fork carrying one
patch (`nsDocShell.cpp`) that adds a chrome-privileged, filesystem-driven JS
injection channel. It is what `grabmore` and the `agent.browser` control drive
instead of geckodriver: injected JS runs as the system principal, so CSP,
CORS, and anti-automation surfaces don't apply.

You normally don't touch these files by hand — the **`agent.browser_builder`**
wizard runs every stage for you and streams the log. This README is for when
you want to run a stage yourself, or understand what the wizard is doing.

## Files (materialized from library assets)

- `build.sh` — the stage driver (download → extract → patch → deps → build).
- `apply-patches.sh` — applies everything under `patches/` to the source tree.
- `mozconfig` — the build configuration copied into the source tree.
- `nsDocShell_v2_block.cpp` — the v2 mechanism block. The wizard's
  `apply_patch` inserts it into the extracted `nsDocShell.cpp` by anchored
  text insertion (before `nsDocShell::EndPageLoad`) and writes the result to
  `patches/`. **`nsDocShell.h` is not patched** — v2's functions are
  file-static free functions needing only public `GetWindow()`/`GetDocument()`.
- `.gitignore` — keeps `work/` (source + objdir, tens of GB) out of git.
- `patches/` — generated; the anchored patch lands here for `apply-patches.sh`.
- `work/` — generated; the tarball, extracted source, and `obj-firefox/`.

## Running a stage by hand

    ./build.sh              # deps + download + extract + patch + build
    ./build.sh download     # one or more named stages, in order
    ./build.sh extract patch build

Stages: `download`, `extract`, `patch`, `configure`, `deps`, `build`,
`package`, `run`, `clean`. Override defaults by exporting first, e.g.
`FIREFOX_VERSION=128.13.0esr JOBS=8 ./build.sh build`.

The built binary lands at:

    work/firefox-<version>/obj-firefox/dist/bin/firefox

## Placing it for grabmore

`grabmore` looks for the browser at a fixed path (`NOOBSCAPE_BIN`, default
`/noobscape/bin/firefox`). The wizard's **install** step points that path at
the build — either a symlink to `dist/bin/firefox` (light; Firefox finds
`libxul` beside the real binary) or a copy of the whole `dist/bin` tree
(survives a `clean`). Retarget by changing `NOOBSCAPE_BIN`.

## Caveats

- A full build needs tens of GB of disk and anywhere from ~30 min to hours.
- `deps` runs `mach bootstrap`, which installs system build prerequisites.
- The patch compiles only against a real Gecko tree; the drift-prone spots
  (`AutoJSAPI`, `JS::Evaluate`, `JS_Stringify`) may need a touch-up on a new
  ESR. If a build ever proves `GetWindow()`/`GetDocument()` non-public, the
  fallback is to make them members and add a small `nsDocShell.h` patch.
