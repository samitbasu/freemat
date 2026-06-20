#!/usr/bin/env bash
# Time the conformance corpus on the ORIGINAL FreeMat (C++) interpreter.
#
# Runs one FreeMat process per covered directory (process startup is excluded
# from every measurement — see fm_timing_driver.m), each writing a CSV chunk,
# then concatenates them into freemat_timings.csv next to this script.
#
# Usage: run_freemat_timing.sh [budget_s] [max_reps] [dir ...]
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORPUS="$(cd "$HERE/../data/tests" && pwd)"
FM="${FREEMAT_BIN:-/tmp/fmbuild/src/FreeMat}"
TBROOT="${FREEMAT_TOOLBOX:-/home/samitbasu/Devel/freemat/FreeMat/toolbox}"

BUDGET="${1:-0.1}"; [ $# -gt 0 ] && shift || true
MAXREPS="${1:-200000}"; [ $# -gt 0 ] && shift || true

ALL_DIRS=(array binary constants elementary flow freemat functions handle \
          inspection io operators random signal sparse string suite \
          transforms typecast variables)
DIRS=("$@"); [ ${#DIRS[@]} -eq 0 ] && DIRS=("${ALL_DIRS[@]}")

[ -x "$FM" ] || { echo "FreeMat binary not found/executable: $FM" >&2; exit 1; }

# FreeMat's -p is non-recursive, so every toolbox subdir must be listed (as
# run-test does). The test dir, shared _helpers, and this driver dir go first.
TBPATH="$(ls -d "$TBROOT"/*/ | sed 's:/$::' | paste -sd:)"
OUT="$HERE/freemat_timings.csv"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "FreeMat: $FM"
echo "budget=${BUDGET}s max_reps=$MAXREPS dirs=${#DIRS[@]}"
echo "dir,name,outcome,reps,ns_per_iter" > "$OUT"

for d in "${DIRS[@]}"; do
  dpath="$CORPUS/$d"
  [ -d "$dpath" ] || { echo "  skip $d (no dir)"; continue; }
  chunk="$TMP/$d.csv"
  PTH="$dpath:$CORPUS/_helpers:$HERE:$TBPATH"
  printf '  timing %-12s ... ' "$d"
  # Per-directory timeout guards against a pathological test wedging the run.
  if timeout 1800 "$FM" -e -nogui -nogreet -noX -p "$PTH" \
        -f "fm_timing_driver('$dpath','$d',$BUDGET,$MAXREPS,'$chunk')" \
        >/dev/null 2>&1 && [ -f "$chunk" ]; then
    tail -n +2 "$chunk" >> "$OUT"
    echo "$(($(wc -l < "$chunk") - 1)) tests"
  else
    echo "FAILED (no output)"
  fi
done

echo "Wrote $OUT ($(($(wc -l < "$OUT") - 1)) rows)"
