#!/bin/sh
# Public-only, optional benchmark. Pass STEP results from the public helper.
set -eu
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
out=${1:-artifacts/atlas-scheduling}
if [ "$#" -gt 0 ]; then shift; fi
mkdir -p "$out"
for variant in group-workers partition; do
  exe="$out/$variant"
  "${CXX:-c++}" -std=c++17 -O2 -DFABSTEP_LARGE_STL_SOURCE -isystem "$sdk/include/opencascade" "$src/qualify-atlas-$variant.cpp" -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKDESTEP -lTKXSBase -lTKBO -lTKPrim -lTKGeomAlgo -lTKTopAlgo -lTKBRep -lTKGeomBase -lTKG3d -lTKMath -lTKernel -o "$exe"
  case "$variant" in group-workers) modes="single intra two two-sorted";; partition) modes="base split slabs both";; esac
  if [ "$variant" = partition ]; then
    for transfer in '' --roundtrip; do
      "$exe" angled-slabs oracle $transfer > "$out/slab-oracle.log"
      "$exe" angled-slabs slabs $transfer > "$out/slabs.log"
      grep -q 'separated_pairs=1 removed_members=2' "$out/slabs.log"
      for test_mode in oracle slabs; do
        if "$exe" near-slabs "$test_mode" $transfer > "$out/near.log" 2>&1; then
          echo "Unresolved near contact accepted" >&2; exit 1
        fi
        grep -q 'auto-intersections' "$out/near.log"
      done
    done
  fi
  for mode in $modes; do
    for roundtrip in '' --roundtrip; do
      "$exe" disjoint "$mode" $roundtrip > "$out/control.log"
      if "$exe" crossing "$mode" $roundtrip > "$out/crossing.log" 2>&1; then
        echo "Crossing accepted by $variant $mode" >&2; exit 1
      fi
      grep -q 'auto-intersections' "$out/crossing.log"
    done
    if "$exe" open-wire "$mode" > "$out/open-wire.log" 2>&1; then
      echo "Open wire accepted by $variant $mode" >&2; exit 1
    fi
    grep -q 'auto-intersections' "$out/open-wire.log"
    for path in "$@"; do
      for run in 1 2 3; do
        /usr/bin/time -lp "$exe" "$path" "$mode"
      done
      "$exe" "$path" "$mode" --roundtrip
    done
  done
done

if [ "$#" -gt 0 ]; then
  python3 - "$out/group-workers" "$1" <<'PYTHON'
import os
import select
import subprocess
import sys
import time

with subprocess.Popen(
    [sys.argv[1], sys.argv[2], "two"],
    stdout=subprocess.PIPE, env={**os.environ, "FABSTEP_BENCH_READY": "1"}
) as process:
    try:
        if not select.select([process.stdout], [], [], 30)[0] or process.stdout.readline().strip() != b"two-active":
            raise RuntimeError("Two native controls did not start")
        start = time.monotonic()
        process.terminate()
        if process.wait(timeout=5) >= 0:
            raise RuntimeError("Expected signal cancellation")
        print(f"Concurrent group process reaped in {time.monotonic() - start:.4f} s")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
PYTHON
fi
