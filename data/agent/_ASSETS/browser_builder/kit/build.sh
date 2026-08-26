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
#   deps      Install build prerequisites via `mach bootstrap`
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
# Configuration
# --------------------------------------------------------------------------
FIREFOX_VERSION="${FIREFOX_VERSION:-128.0esr}"
WORKDIR="${WORKDIR:-$(pwd)/work}"
JOBS="${JOBS:-$(nproc 2>/dev/null || echo 4)}"
MOZ_FTP_BASE="${MOZ_FTP_BASE:-https://ftp.mozilla.org/pub/firefox/releases}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PATCH_DIR="${REPO_ROOT}/patches"
MOZCONFIG_SRC="${REPO_ROOT}/mozconfig"

TARBALL="firefox-${FIREFOX_VERSION}.source.tar.xz"
TARBALL_URL="${MOZ_FTP_BASE}/${FIREFOX_VERSION}/source/${TARBALL}"
CHECKSUMS_URL="${MOZ_FTP_BASE}/${FIREFOX_VERSION}/SHA256SUMS"
SRC_DIR="${WORKDIR}/firefox-${FIREFOX_VERSION}"

# --------------------------------------------------------------------------
# Logging helpers
# --------------------------------------------------------------------------
log()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mWARN:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

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
    log "Bootstrapping build dependencies (this may prompt / take a while)"
    ( cd "${SRC_DIR}" && ./mach bootstrap --application-choice browser --no-interactive )
}

stage_build() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; run the 'extract' stage first"
    stage_configure
    log "Building Firefox with ${JOBS} jobs (grab a coffee — this takes a while)"
    ( cd "${SRC_DIR}" && MOZCONFIG="${SRC_DIR}/mozconfig" ./mach build -j"${JOBS}" )
    log "Build complete. Binary: ${SRC_DIR}/obj-firefox/dist/bin/firefox"
}

stage_package() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; build first"
    log "Packaging distributable build"
    ( cd "${SRC_DIR}" && MOZCONFIG="${SRC_DIR}/mozconfig" ./mach package )
    log "Package written under ${SRC_DIR}/obj-firefox/dist/"
}

stage_run() {
    [[ -d "${SRC_DIR}" ]] || die "Source tree missing; build first"
    log "Launching the freshly built Firefox"
    ( cd "${SRC_DIR}" && MOZCONFIG="${SRC_DIR}/mozconfig" ./mach run )
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
