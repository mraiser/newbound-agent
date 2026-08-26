#!/usr/bin/env bash
#
# apply-patches.sh — Apply every real patch under patches/ to a source tree.
#
# Used by build.sh, but can also be run standalone:
#
#   PATCH_DIR=./patches SRC_DIR=./work/firefox-128.0esr ./apply-patches.sh
#
# Patches are applied in lexical filename order (name them 0001-, 0002-, ...).
# A file that contains no unified-diff hunks is treated as a placeholder and
# skipped with a warning, so the build still succeeds before a real patch is
# dropped in.
#
set -euo pipefail

PATCH_DIR="${PATCH_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/patches}"
SRC_DIR="${SRC_DIR:?Set SRC_DIR to the extracted Firefox source tree}"

log()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mWARN:\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

[[ -d "${SRC_DIR}" ]]   || die "Source tree not found: ${SRC_DIR}"
[[ -d "${PATCH_DIR}" ]] || die "Patch directory not found: ${PATCH_DIR}"

shopt -s nullglob
patches=( "${PATCH_DIR}"/*.patch "${PATCH_DIR}"/*.diff )
shopt -u nullglob

if [[ ${#patches[@]} -eq 0 ]]; then
    warn "No patches found in ${PATCH_DIR}"
    exit 0
fi

# Sort by filename so numeric prefixes control ordering.
IFS=$'\n' patches=( $(printf '%s\n' "${patches[@]}" | sort) )
unset IFS

applied=0
for patch in "${patches[@]}"; do
    name="$(basename "${patch}")"

    # A patch with no hunk header (@@) is a placeholder — skip it.
    if ! grep -qE '^@@ ' "${patch}"; then
        warn "Skipping placeholder patch (no diff hunks): ${name}"
        continue
    fi

    # Verify the patch applies cleanly before touching the tree.
    if patch -p1 --dry-run --directory="${SRC_DIR}" < "${patch}" >/dev/null 2>&1; then
        log "Applying ${name}"
        patch -p1 --directory="${SRC_DIR}" < "${patch}"
        applied=$((applied + 1))
    elif patch -p1 --dry-run --reverse --directory="${SRC_DIR}" < "${patch}" >/dev/null 2>&1; then
        warn "Already applied, skipping: ${name}"
    else
        die "Patch does not apply cleanly: ${name} (run 'patch -p1 --dry-run' for details)"
    fi
done

log "Applied ${applied} patch(es) to ${SRC_DIR}"
