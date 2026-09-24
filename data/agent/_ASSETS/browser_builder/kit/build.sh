#!/usr/bin/env bash
#
# build.sh — Download, patch, and build Firefox 128 ESR from source.
#
# This script fetches the official Firefox ESR source tarball, applies the
# local patches under patches/, configures the build via ./mozconfig, and
# compiles the native Firefox application with mach.
#
# Usage:
#   ./build.sh              # run every stage: deps -> download -> patch -> build
#   ./build.sh <stage>...   # run only the named stage(s), in order given
#
# Stages:
#   deps      Fetch prebuilt toolchains from Mozilla CI into ~/.mozbuild
#   download  Download and verify the source tarball
#   extract   Extract the tarball into the build directory
#   patch     Apply everything under patches/ to the extracted tree
#   configure Copy ./mozconfig into the source tree
#   build     Compile Firefox (`mach build`)
#   package   Produce a distributable package (`mach package`)
#   run       Launch the freshly built browser (`mach run`)
#   clean     Remove the downloaded tarball and extracted tree
#
# Configuration (override by exporting before running, e.g.
#   FIREFOX_VERSION=128.13.0esr ./build.sh):
#
#   FIREFOX_VERSION   ESR version to build            (default: 128.0esr)
#   WORKDIR           Where source + objdir live      (default: ./work)
#   JOBS              Parallel compile jobs           (default: nproc)
#   MOZ_FTP_BASE      Mozilla release mirror base URL
#
set -euo pipefail

# --------------------------------------------------------------------------
# Guaranteed tool PATH
# --------------------------------------------------------------------------
# The newbound server process runs with an EMPTY PATH, and run_stage spawns
# this script with no env_clear, so `od`/`env`/`dirname`/`uname` may not be
# findable — which silently breaks the mach interpreter resolver below (its
# `od` e_machine check then fails for EVERY candidate). Prepend a minimal,
# always-present tool dir (Nix coreutils) plus the conventional locations so
# the script is self-sufficient regardless of the inherited environment.
for _td in /run/current-system/sw/bin /nix/store/*-coreutils-*/bin /usr/bin /bin /usr/local/bin; do
    [[ -d "$_td" ]] && PATH="${_td}${PATH:+:${PATH}}"
done
export PATH

# Strip loader INJECTION inherited from the launcher. `stdbuf -oL -eL` wraps
# this script and injects LD_PRELOAD=.../libstdbuf.so; left set it leaks into the
# interpreter probes below (and into mach): the glibc-2.38 libstdbuf.so fails to
# load against the server's glibc-2.37 LD_LIBRARY_PATH and EVERY candidate is
# wrongly rejected. LD_PRELOAD must go entirely. LD_LIBRARY_PATH is NOT unset —
# the staged Mozilla clang (CC, below) links libxml2.so.2 from the compile
# sysroot and needs a search path for it; instead we REBUILD it to just the
# sysroot's lib dir, dropping the server's poisoned glibc-2.37 entry.
unset LD_PRELOAD
unset LD_LIBRARY_PATH

# The server's environment is COMPLETELY empty — no HOME either — and `set -u`
# makes ${HOME} at MOZBUILD below a fatal "unbound variable". Seed HOME from the
# passwd entry when absent so the script never depends on inherited state.
if [[ -z "${HOME:-}" ]]; then
    # Pure-bash uid + /etc/passwd lookup — NO subprocess (id/getent/cut), so the
    # glibc-2.37 LD_LIBRARY_PATH that stdbuf's LD_PRELOAD re-injects can't break
    # the lookup. $UID is a bash builtin; /etc/passwd fields are colon-separated
    # with the home dir in field 6.
    HOME="/tmp"
    if [[ -n "${UID:-}" && -r /etc/passwd ]]; then
        while IFS= read -r _line || [[ -n "${_line}" ]]; do
            IFS=':' read -r -a _f <<< "${_line}"
            if [[ "${_f[2]:-}" == "${UID}" ]]; then HOME="${_f[5]:-/tmp}"; break; fi
        done < /etc/passwd
    fi
    export HOME
    unset _line _f
fi

# --------------------------------------------------------------------------
# Configuration
# --------------------------------------------------------------------------
FIREFOX_VERSION="${FIREFOX_VERSION:-128.0esr}"
WORKDIR="${WORKDIR:-$(pwd)/work}"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"
MOZ_FTP_BASE="${MOZ_FTP_BASE:-https://ftp.mozilla.org/pub/firefox/releases}"

# --- mach interpreter pinning (Nix host) ----------------------------------
# mach's venv bootstrap needs a NATIVE x86-64 CPython <= 3.11, and mach runs
# as `#!/usr/bin/env python3`, honoring the first python3 on PATH. This host's
# ambient python3 is 3.12 (rejected by mach's site.py) and its store 3.11
# builds are aarch64 under qemu binfmt (cannot dlopen the x86-64
# libglean_ffi.so). Resolve MACH_PYTHON to a NATIVE <= 3.11 and prepend its
# dir to PATH at every ./mach call. Override by exporting MACH_PYTHON.
# LD_LIBRARY_PATH is scrubbed: its Nix-profile entries (libffi needing glibc
# 2.38) poison this glibc-2.37 interpreter, and mach manages its own env.
unset LD_LIBRARY_PATH
MACH_PYTHON="${MACH_PYTHON:-}"
if [[ -z "${MACH_PYTHON}" ]]; then
    # Pin the known-good NATIVE x86-64 3.10 FIRST: this host's only native
    # <=3.11. Its store 3.11 builds are aarch64 (e_machine b700) under qemu
    # binfmt and are correctly rejected by the od check below. Then widen the
    # search; a 3.11 beats 3.10 when a native one exists.
    for _c in /nix/store/a5k7x5mn7i7rcji4n99mwiqhmgjdzxmk-python3-3.10.12/bin/python3.10 \
              /nix/store/*-python3-3.11*/bin/python3.11 \
              /nix/store/*-python3-3.10*/bin/python3.10 \
              /nix/store/*-python3-3.9*/bin/python3.9 \
              python3.11 python3.10 python3.9; do
        _r="$(command -v "$_c" 2>/dev/null || true)"; [[ -n "$_r" ]] || continue
        # Ask the interpreter ITSELF: native arch AND version <= 3.11 AND working
        # ctypes. This single probe rejects the aarch64 3.11 builds (they fail to
        # execute natively / report wrong platform) and the ambient 3.12, and it
        # spawns nothing else — LD_LIBRARY_PATH is emptied inline so the server's
        # glibc-2.37 can't poison a 2.38-needing interpreter. (The earlier
        # pure-bash ELF byte-read returned EMPTY under `stdbuf`/setsid, wrongly
        # rejecting every candidate — the interpreter probe has no such fragility.)
        if LD_LIBRARY_PATH= "$_r" -c 'import sys,ctypes,platform;raise SystemExit(0 if (sys.version_info[:2]<=(3,11) and platform.machine()=="x86_64") else 1)' 2>/dev/null; then
            MACH_PYTHON="$_r"; break
        fi
    done
    # NEVER fall back to bare `python3`: on this host that is the ambient
    # 3.12, which mach rejects with the misleading 'not in the subpath' crash.
    if [[ -z "${MACH_PYTHON}" ]]; then
        echo "build.sh: no NATIVE x86-64 CPython <= 3.11 found for mach." >&2
        echo "  The ambient python3 is $(python3 --version 2>&1) (too new);" >&2
        echo "  install a native python3.11/3.10 or export MACH_PYTHON=/path/to/python." >&2
        exit 1
    fi
fi
MACH_PATH_DIR="$(dirname "${MACH_PYTHON}")"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATCH_DIR="${REPO_ROOT}/patches"
MOZCONFIG_SRC="${REPO_ROOT}/mozconfig"

TARBALL="firefox-${FIREFOX_VERSION}.source.tar.xz"
TARBALL_URL="${MOZ_FTP_BASE}/${FIREFOX_VERSION}/source/${TARBALL}"
CHECKSUMS_URL="${MOZ_FTP_BASE}/${FIREFOX_VERSION}/SHA256SUMS"
SRC_DIR="${WORKDIR}/firefox-${FIREFOX_VERSION}"

MOZBUILD="${MOZBUILD_STATE_PATH:-${HOME}/.mozbuild}"
# Search path (xz/liblzma only) for libxml2's transitive dep during bindgen's
# CDLL(libclang). Carries no libstdc++, so it cannot poison rustc/cargo/curl.
# Resolved dynamically every run (Nix store paths drift across system updates).
if [[ -z "${BINDGEN_LD_PATH:-}" && -n "${MACH_PYTHON:-}" && -d "${MOZBUILD}/clang/lib" ]]; then
    # Test-LOAD each candidate: an xz dir wins only if libclang actually CDLLs
    # with it as the sole LD_LIBRARY_PATH (some store liblzma are 32-bit or have
    # unmet deps of their own). This mirrors exactly what bindgen's check does.
    for _xz in /nix/store/*xz*/lib/liblzma.so.5; do
        [[ -f "$_xz" ]] || continue
        _d="$(dirname "$_xz")"
        if env -i PATH=/usr/bin:/bin LD_LIBRARY_PATH="$_d" "${MACH_PYTHON}" \
            -c "from ctypes import CDLL; CDLL('${MOZBUILD}/clang/lib/libclang.so').clang_getAddressSpace" \
            >/dev/null 2>&1; then
            BINDGEN_LD_PATH="$_d"; break
        fi
    done
fi
BINDGEN_LD_PATH="${BINDGEN_LD_PATH:-}"

# NOTE: the compile sysroot's usr/lib dir carries an OLD libcom_err.so.2
# (e2fsprogs era) and a too-old libstdc++.so.6. Putting that dir on
# LD_LIBRARY_PATH -- whether script-wide or inline on ./mach -- breaks curl,
# cargo, and rustc ('libcom_err.so.2: cannot open shared object file',
# 'GLIBCXX_3.4.32 not found'). So we NEVER put the sysroot on LD_LIBRARY_PATH.
# The staged clang suite gets its libxml2.so.2 via an $ORIGIN/../lib symlink
# instead (see provision_libxml2 in stage_deps), which needs no env at all.
TC_INDEX="https://firefox-ci-tc.services.mozilla.com/api/index/v1/task"
TC_QUEUE="https://firefox-ci-tc.services.mozilla.com/api/queue/v1/task"

# The cbindgen this tree's headers were written for. CI's .latest artifact
# tracks mozilla-central and panics on 128's cbindgen.toml, so this one is
# built from source with cargo and version-gated.
CBINDGEN_VERSION="0.26.0"

# --------------------------------------------------------------------------
# Logging helpers
# --------------------------------------------------------------------------
log()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mWARN:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

# --------------------------------------------------------------------------
# Helpers
# --------------------------------------------------------------------------
# The release tarball is not a VCS checkout, but mach's tooling enumerates
# files through mozversioncontrol's tracked-files finder; outside a repo the
# patterns match nothing. Committing the pristine tree into a throwaway local
# git repo makes the tarball look like the checkout the tools expect.
ensure_git_repo() {
    [[ -d "${SRC_DIR}" ]] || return 0
    [[ -d "${SRC_DIR}/.git" ]] && return 0
    if ! command -v git >/dev/null 2>&1; then
        warn "git not found: mach's file finder needs a VCS checkout; some mach tooling may fail"
        return 0
    fi
    log "Initialising throwaway git repo in ${SRC_DIR} (mach's file finder only sees tracked files)"
    ( cd "${SRC_DIR}" \
        && git init -q \
        && git add -A \
        && git -c user.email=noobscape@localhost -c user.name=noobscape \
               commit -qm "firefox ${FIREFOX_VERSION} source tarball" )
}

# Fetch one prebuilt toolchain from Mozilla CI's cache by NAME (the .latest
# index route), never by digest: `mach bootstrap`'s digest lookup hashes a
# VCS file listing a tarball can never reproduce, so it always misses. These
# are the same artifacts bootstrap would have installed, addressed stably.
fetch_toolchain() {  # <index-name> <dest-dir-under-~/.mozbuild>
    local name="$1" dest="$2"
    if [[ -e "${MOZBUILD}/${dest}" ]]; then
        log "Toolchain already present: ${MOZBUILD}/${dest}"
        return 0
    fi
    local ns="gecko.cache.level-3.toolchains.v3.${name}.latest"
    local tid art
    tid="$(curl -fsSL --retry 3 "${TC_INDEX}/${ns}" \
        | python3 -c 'import json,sys; print(json.load(sys.stdin)["taskId"])')"
    [[ -n "${tid}" ]] || die "No task in the toolchain cache for ${ns}"
    art="$(curl -fsSL --retry 3 "${TC_QUEUE}/${tid}/artifacts" \
        | python3 -c 'import json,sys; print(next(a["name"] for a in json.load(sys.stdin)["artifacts"] if a["name"].startswith("public/build/") and ".tar." in a["name"]))')"
    [[ -n "${art}" ]] || die "No public/build artifact on task ${tid} (${name})"
    log "Fetching ${name}: task ${tid}, ${art}"
    mkdir -p "${MOZBUILD}"
    local tmp="${MOZBUILD}/.fetch-${dest##*/}.tar.zst"
    curl -fSL --retry 3 -o "${tmp}" "${TC_QUEUE}/${tid}/artifacts/${art}"
    tar --zstd -xf "${tmp}" -C "${MOZBUILD}"
    rm -f "${tmp}"
    [[ -e "${MOZBUILD}/${dest}" ]] \
        || die "Extracting ${art} did not produce ${MOZBUILD}/${dest}"
}

# cbindgen is the one tool the CI cache cannot supply: .latest tracks
# mozilla-central and its config parser rejects this older tree. Build the
# pinned version with the box's own cargo, into the exact path configure's
# bootstrap search reads (~/.mozbuild/cbindgen/cbindgen). Version-gated so a
# wrong binary (e.g. an earlier fetched .latest) is replaced, not trusted.
# The staged Mozilla clang suite (clang, llvm-readelf, ...) links libxml2.so.2,
# which ships in the compile sysroot, NOT in the clang toolchain. The binaries
# have RUNPATH $ORIGIN/../lib, so a symlink there lets them run with NO
# LD_LIBRARY_PATH at all. This is the env-independent fix: mach scrubs
# LD_LIBRARY_PATH before configure, and exporting the sysroot path script-wide
# poisons curl (its old libcom_err.so.2 shadows the system krb5's .so.3).
# Idempotent; re-created on every deps run so a re-fetched toolchain self-heals.
# The staged Mozilla clang suite (clang, llvm-*, libclang) is built for a generic
# older glibc and expects its support libs beside it. On this NixOS box the loader
# has NO default search path under mach's scrubbed env, so libclang's deps
# (libstdc++/libgcc_s/libz) and libxml2's transitive dep (liblzma) go unfound and
# bindgen's `CDLL(libclang)` dies with a misleading 'libclang too old'. Fix:
#   * Symlink the needed libs into clang's $ORIGIN/../lib (its RUNPATH), resolved
#     DYNAMICALLY from the Nix store (paths change across system updates), choosing
#     a libstdc++ that provides GLIBCXX_3.4.22+ yet needs no newer glibc than the
#     host's (gcc-12/13 era -- the gcc-15 builds require glibc 2.38 the mach
#     interpreter lacks). libclang rpath then finds them env-free.
#   * RUNPATH is NOT transitive, so libxml2's liblzma still needs a search path:
#     we export BINDGEN_LD_PATH = the xz lib dir ONLY, and set it as LD_LIBRARY_PATH
#     on the ./mach invocations. It carries no libstdc++, so it cannot poison
#     rustc/cargo/curl (which use their own rpath).
# Idempotent and self-healing: re-resolved on every deps run.
_elf64() { readelf -h "$1" 2>/dev/null | grep -q 'Class:.*ELF64'; }
_maxglibc() { readelf -V "$1" 2>/dev/null | grep -oE 'GLIBC_2\.[0-9]+' | sort -V | tail -1; }

# Find a 64-bit libstdc++ providing GLIBCXX_3.4.22 whose glibc requirement the
# host satisfies (reject any needing GLIBC_2.38+, which the 2.37 mach python lacks).
_find_libstdcxx() {
    local d rf
    for d in /nix/store/*gcc*-lib/lib64 /nix/store/*gcc*-lib/lib; do
        [[ -f "${d}/libstdc++.so.6" ]] || continue
        rf="$(readlink -f "${d}/libstdc++.so.6")"
        _elf64 "$rf" || continue
        strings "$rf" 2>/dev/null | grep -q 'GLIBCXX_3.4.22' || continue
        [[ "$(_maxglibc "$rf")" == "GLIBC_2.38" ]] && continue
        echo "$rf"; return 0
    done
    return 1
}
_find_lib() { # <soname-glob> e.g. 'lib/liblzma.so.5' under /nix/store/*xz*
    local pat="$1" c rf
    shift
    for c in "$@"; do
        [[ -f "$c" ]] || continue
        rf="$(readlink -f "$c")"
        _elf64 "$rf" && { echo "$rf"; return 0; }
    done
    return 1
}

provision_libxml2() {
    local clangbin="${MOZBUILD}/clang/bin"
    local clanglib="${MOZBUILD}/clang/lib"
    local sysusr="${MOZBUILD}/sysroot-$(uname -m)-linux-gnu/usr/lib/$(uname -m)-linux-gnu"
    [[ -d "${clanglib}" ]] || return 0

    # libxml2 (clang's direct need) into clang's rpath dir.
    local real="${sysusr}/libxml2.so.2.9.1"
    if [[ -f "${real}" && ! -e "${clanglib}/libxml2.so.2" ]]; then
        ln -sf "${real}" "${clanglib}/libxml2.so.2.9.1"
        ln -sf "libxml2.so.2.9.1" "${clanglib}/libxml2.so.2"
    fi

    # libstdc++ + libgcc_s into clang's rpath dir (libclang direct deps).
    local cxx; cxx="$(_find_libstdcxx || true)"
    if [[ -n "${cxx}" && ! -e "${clanglib}/libstdc++.so.6" ]]; then
        ln -sf "${cxx}" "${clanglib}/$(basename "${cxx}")"
        ln -sf "$(basename "${cxx}")" "${clanglib}/libstdc++.so.6"
        local gdir; gdir="$(dirname "${cxx}")"
        [[ -f "${gdir}/libgcc_s.so.1" ]] && ln -sf "$(readlink -f "${gdir}/libgcc_s.so.1")" "${clanglib}/libgcc_s.so.1"
    fi
    # libz + liblzma into clang's rpath dir (libLLVM/libxml2 needs).
    local lz xz
    lz="$(_find_lib x /nix/store/*zlib*/lib/libz.so.1 || true)"
    if [[ -n "${lz}" && ! -e "${clanglib}/libz.so.1" ]]; then
        ln -sf "${lz}" "${clanglib}/$(basename "${lz}")"; ln -sf "$(basename "${lz}")" "${clanglib}/libz.so.1"
    fi
    xz="$(_find_lib x /nix/store/*xz*/lib/liblzma.so.5 || true)"
    if [[ -n "${xz}" && ! -e "${clanglib}/liblzma.so.5" ]]; then
        ln -sf "${xz}" "${clanglib}/$(basename "${xz}")"; ln -sf "$(basename "${xz}")" "${clanglib}/liblzma.so.5"
    fi
    # RUNPATH is not transitive: libxml2's liblzma needs a real search path.
    # Export the xz dir (no libstdc++ in it) for the ./mach LD_LIBRARY_PATH.
    if [[ -n "${xz}" ]]; then BINDGEN_LD_PATH="$(dirname "${xz}")"; fi

    # A `readelf` on configure's PATH (clang_search_path includes clang/bin).
    if [[ -x "${clangbin}/llvm-readelf" && ! -e "${clangbin}/readelf" ]]; then
        ln -sf llvm-readelf "${clangbin}/readelf"
        log "Shimmed readelf -> llvm-readelf in clang/bin"
    fi
    log "Provisioned clang toolchain support libs (libxml2/libstdc++/libz/liblzma)"
}

# Configure needs rustc/cargo/rustdoc on PATH, all runnable in mach's scrubbed
# env (no LD_LIBRARY_PATH). This box's rust lives in the Nix store, and the
# system profile exposes cargo but NOT rustc. Build a .rustbin shim dir of
# symlinks to NATIVE, env-clean binaries (verified with `env -i`), resolved
# dynamically so a Nix store path change self-heals on the next deps run.
provision_rust() {
    local shim="${MOZBUILD}/.rustbin"
    mkdir -p "${shim}"
    # Candidate sources: whatever is on PATH first, then native store globs.
    local rustc_bin="" cargo_bin="" rustdoc_bin="" c
    for c in $(command -v rustc 2>/dev/null) /nix/store/*rustc-wrapper*/bin/rustc /nix/store/*rustc-1.*/bin/rustc; do
        [[ -x "$c" ]] || continue
        if env -i "$c" --version >/dev/null 2>&1; then rustc_bin="$c"; break; fi
    done
    for c in $(command -v cargo 2>/dev/null) /nix/store/*cargo-1.*/bin/cargo; do
        [[ -x "$c" ]] || continue
        # reject the Nix system-path indirection (broken outside the profile)
        case "$(readlink -f "$c" 2>/dev/null)" in *system-path*) continue;; esac
        if env -i "$c" --version >/dev/null 2>&1; then cargo_bin="$c"; break; fi
    done
    if [[ -n "${rustc_bin}" ]]; then
        local rdir; rdir="$(dirname "${rustc_bin}")"
        [[ -x "${rdir}/rustdoc" ]] && rustdoc_bin="${rdir}/rustdoc"
        ln -sf "${rustc_bin}" "${shim}/rustc"
        [[ -n "${rustdoc_bin}" ]] && ln -sf "${rustdoc_bin}" "${shim}/rustdoc"
    fi
    [[ -n "${cargo_bin}" ]] && ln -sf "${cargo_bin}" "${shim}/cargo"
    if [[ -x "${shim}/rustc" && -x "${shim}/cargo" ]]; then
        log "Rust toolchain shimmed in ${shim} (rustc: ${rustc_bin})"
    else
        warn "no env-clean native rustc/cargo found; configure's rust check may fail"
    fi
}

# Configure looks up assorted host tools BY NAME on PATH (unzip, zip, m4, awk,
# tar, ...). On this NixOS box they live in the store, not on the sanitized PATH.
# Build a .hostbin shim of symlinks to native, env-clean binaries, resolved
# dynamically. Only tools that resolve are linked; configure's alternatives
# (m4|gm4, gawk|mawk|nawk) are covered by the one real binary we find.
provision_hosttools() {
    local shim="${MOZBUILD}/.hostbin"
    mkdir -p "${shim}"
    local t c
    for t in unzip zip m4 gawk awk tar gzip bzip2 xz make gmake; do
        [[ -e "${shim}/${t}" ]] && continue
        for c in $(command -v "${t}" 2>/dev/null) /nix/store/*${t}*/bin/${t}; do
            [[ -x "$c" ]] || continue
            case "$(readlink -f "$c" 2>/dev/null)" in *system-path*) continue;; esac
            if env -i "$c" --version >/dev/null 2>&1 || env -i "$c" --help >/dev/null 2>&1; then
                ln -sf "$c" "${shim}/${t}"; break
            fi
        done
    done
    # A no-op dump_syms: this tarball build generates no Breakpad symbols
    # (crashreporter disabled), but moz.configure's DUMP_SYMS check would
    # otherwise trigger a VCS-dependent `mach artifact toolchain` fetch that
    # fails on a tarball ('No such remote origin'). An explicit program on PATH
    # (plus DUMP_SYMS exported in mozconfig) makes configure skip the bootstrap.
    if [[ ! -e "${shim}/dump_syms" ]]; then
        printf '#!/bin/sh\n# no-op dump_syms stub (tarball build, no Breakpad symbols)\nexit 0\n' > "${shim}/dump_syms"
        chmod +x "${shim}/dump_syms"
    fi
    log "Host tools shimmed in ${shim} ($(ls "${shim}" | tr '\n' ' '))"
}

provision_cbindgen() {
    local dest="${MOZBUILD}/cbindgen/cbindgen"
    if [[ -x "${dest}" ]] && "${dest}" --version 2>/dev/null | grep -q "${CBINDGEN_VERSION}"; then
        log "cbindgen ${CBINDGEN_VERSION} already present: ${dest}"
        return 0
    fi
    command -v cargo >/dev/null 2>&1 || die "cargo not found; needed to build cbindgen ${CBINDGEN_VERSION}"
    log "Building cbindgen ${CBINDGEN_VERSION} with cargo (CI's .latest artifact is too new for this tree)"
    cargo install --locked --quiet --version "${CBINDGEN_VERSION}" \
        --root "${MOZBUILD}/cbindgen-cargo" cbindgen
    rm -rf "${MOZBUILD}/cbindgen"
    mkdir -p "${MOZBUILD}/cbindgen"
    cp "${MOZBUILD}/cbindgen-cargo/bin/cbindgen" "${dest}"
    "${dest}" --version | grep -q "${CBINDGEN_VERSION}" \
        || die "built cbindgen reports the wrong version: $("${dest}" --version)"
}

# --------------------------------------------------------------------------
# Stages
# --------------------------------------------------------------------------
stage_download() {
    mkdir -p "${WORKDIR}"
    local dest="${WORKDIR}/${TARBALL}"

    if [[ -f "${dest}" ]]; then
        log "Tarball already present: ${dest}"
    else
        log "Downloading ${TARBALL_URL}"
        curl -fSL --retry 4 --retry-delay 2 -o "${dest}.part" "${TARBALL_URL}"
        mv "${dest}.part" "${dest}"
    fi

    log "Verifying SHA256 checksum"
    if command -v sha256sum >/dev/null 2>&1; then
        local expected
        expected="$(curl -fSL "${CHECKSUMS_URL}" 2>/dev/null \
            | awk -v f="source/${TARBALL}" '$2 == f {print $1}' || true)"
        if [[ -n "${expected}" ]]; then
            local actual
            actual="$(sha256sum "${dest}" | awk '{print $1}')"
            [[ "${expected}" == "${actual}" ]] \
                || die "Checksum mismatch: expected ${expected}, got ${actual}"
            log "Checksum OK (${actual})"
        else
            warn "Could not fetch checksum for ${TARBALL}; skipping verification"
        fi
    else
        warn "sha256sum not available; skipping checksum verification"
    fi
}

stage_extract() {
    [[ -f "${WORKDIR}/${TARBALL}" ]] || die "Tarball missing; run the 'download' stage first"
    if [[ -d "${SRC_DIR}" ]]; then
        log "Source tree already extracted at ${SRC_DIR}"
        ensure_git_repo
        return
    fi
    log "Extracting ${TARBALL}"
    mkdir -p "${WORKDIR}"
    tar -xf "${WORKDIR}/${TARBALL}" -C "${WORKDIR}"
    # The tarball unpacks to firefox-<version>/ — normalise if it differs.
    if [[ ! -d "${SRC_DIR}" ]]; then
        local extracted
        extracted="$(find "${WORKDIR}" -maxdepth 1 -type d -name 'firefox-*' | head -n1)"
        [[ -n "${extracted}" ]] && mv "${extracted}" "${SRC_DIR}"
    fi
    [[ -d "${SRC_DIR}" ]] || die "Extraction did not produce ${SRC_DIR}"
    ensure_git_repo
}

stage_patch() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; run the 'extract' stage first"
    log "Applying patches from ${PATCH_DIR}"
    PATCH_DIR="${PATCH_DIR}" SRC_DIR="${SRC_DIR}" "${REPO_ROOT}/apply-patches.sh"
}

stage_configure() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; run the 'extract' stage first"
    [[ -f "${MOZCONFIG_SRC}" ]] || die "mozconfig not found at ${MOZCONFIG_SRC}"
    log "Installing mozconfig into source tree"
    cp "${MOZCONFIG_SRC}" "${SRC_DIR}/mozconfig"
}

stage_deps() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; run the 'extract' stage first"
    ensure_git_repo
    # `mach bootstrap` is unusable in this environment: its toolchain path
    # computes cache digests a tarball can never reproduce, and its system
    # package path needs an interactive sudo. Fetch the same CI artifacts it
    # would have installed, into the same ~/.mozbuild layout that mozconfig's
    # --enable-bootstrap=no-update reads.
    local arch
    arch="$(uname -m)"
    log "Fetching prebuilt toolchains for ${arch} into ${MOZBUILD}"
    # fetch_toolchain parses JSON with `python3`, but the sanitized PATH above
    # has no python3 (NixOS sw profile ships none). Expose the already-resolved
    # NATIVE MACH_PYTHON as `python3` for this stage, so the cache lookups run
    # under the same interpreter mach itself uses.
    local _pybin="${MOZBUILD}/.pybin"
    mkdir -p "${_pybin}"
    ln -sf "${MACH_PYTHON}" "${_pybin}/python3"
    PATH="${_pybin}:${PATH}"
    case "${arch}" in
        aarch64|arm64)
            fetch_toolchain "linux64-aarch64-clang-19"   "clang"
            fetch_toolchain "linux64-aarch64-node-22"    "node"
            fetch_toolchain "linux64-aarch64-pkgconf"    "pkgconf"
            fetch_toolchain "linux64-aarch64-pkgconf"    "pkgconf"
            fetch_toolchain "sysroot-aarch64-linux-gnu"  "sysroot-aarch64-linux-gnu"
            [[ -d "${MOZBUILD}/sysroot-aarch64-linux-gnu/usr/include/gtk-3.0" ]] \
                || die "sysroot lacks gtk-3.0 headers; configure cannot build a desktop browser against it"
            ;;
        x86_64)
            fetch_toolchain "linux64-clang-19"           "clang"
            fetch_toolchain "linux64-node-22"            "node"
            fetch_toolchain "linux64-nasm"               "nasm"
            fetch_toolchain "linux64-pkgconf"            "pkgconf"
            fetch_toolchain "linux64-pkgconf"            "pkgconf"
            fetch_toolchain "sysroot-x86_64-linux-gnu"   "sysroot-x86_64-linux-gnu"
            [[ -d "${MOZBUILD}/sysroot-x86_64-linux-gnu/usr/include/gtk-3.0" ]] \
                || die "sysroot lacks gtk-3.0 headers; configure cannot build a desktop browser against it"
            ;;
        *)
            die "No toolchain mapping for ${arch}"
            ;;
    esac
    provision_libxml2
    provision_rust
    provision_hosttools
    provision_cbindgen
    log "Toolchains ready (mozconfig --enable-bootstrap=no-update picks them up)"
}

stage_build() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; run the 'extract' stage first"
    stage_configure
    log "Building Firefox with ${JOBS} jobs (grab a coffee — this takes a while)"
    ( cd "${SRC_DIR}" && PATH="${MACH_PATH_DIR}:${MOZBUILD}/.rustbin:${MOZBUILD}/.hostbin:${PATH}" MOZCONFIG="${SRC_DIR}/mozconfig" env ${BINDGEN_LD_PATH:+LD_LIBRARY_PATH="${BINDGEN_LD_PATH}"} "${MACH_PYTHON}" ./mach build -j"${JOBS}" )
    log "Build complete. Binary: ${SRC_DIR}/obj-firefox/dist/bin/firefox"
}

stage_package() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; build first"
    log "Packaging distributable build"
    ( cd "${SRC_DIR}" && PATH="${MACH_PATH_DIR}:${MOZBUILD}/.rustbin:${MOZBUILD}/.hostbin:${PATH}" MOZCONFIG="${SRC_DIR}/mozconfig" env ${BINDGEN_LD_PATH:+LD_LIBRARY_PATH="${BINDGEN_LD_PATH}"} "${MACH_PYTHON}" ./mach package )
    log "Package written under ${SRC_DIR}/obj-firefox/dist/"
}

stage_run() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; build first"
    log "Launching the freshly built Firefox"
    ( cd "${SRC_DIR}" && PATH="${MACH_PATH_DIR}:${MOZBUILD}/.rustbin:${MOZBUILD}/.hostbin:${PATH}" MOZCONFIG="${SRC_DIR}/mozconfig" env ${BINDGEN_LD_PATH:+LD_LIBRARY_PATH="${BINDGEN_LD_PATH}"} "${MACH_PYTHON}" ./mach run )
}

stage_clean() {
    log "Removing ${SRC_DIR} and ${WORKDIR}/${TARBALL}"
    rm -rf "${SRC_DIR}"
    rm -f "${WORKDIR}/${TARBALL}"
}

# --------------------------------------------------------------------------
# Driver
# --------------------------------------------------------------------------
run_all() {
    stage_download
    stage_extract
    stage_patch
    stage_deps
    stage_build
}

main() {
    log "Firefox version : ${FIREFOX_VERSION}"
    log "Work directory  : ${WORKDIR}"
    log "Parallel jobs   : ${JOBS}"

    if [[ $# -eq 0 ]]; then
        run_all
        return
    fi

    for stage in "$@"; do
        case "${stage}" in
            deps)      stage_deps ;;
            download)  stage_download ;;
            extract)   stage_extract ;;
            patch)     stage_patch ;;
            configure) stage_configure ;;
            build)     stage_build ;;
            package)   stage_package ;;
            run)       stage_run ;;
            clean)     stage_clean ;;
            all)       run_all ;;
            *)         die "Unknown stage: ${stage}" ;;
        esac
    done
}

main "$@"
