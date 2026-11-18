#!/bin/sh
# Debug OCCT laboratory. All model-specific input/output stays local.
set -eu
[ "$#" = 3 ] || { echo 'Use INPUT.step LETTERING.txt OUTPUT_DIR or --frontier INPUT.step OUTPUT_DIR' >&2; exit 2; }
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
out=$3
mkdir -p "$out"
sdk=${OCCT_ROOT:-}
if [ -z "$sdk" ]; then
  for path in "${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade" /usr/local/opt/opencascade /usr/local; do
    if [ -f "$path/include/opencascade/Standard_Version.hxx" ]; then sdk=$path; break; fi
  done
fi
if ! grep -Fqx '#define OCC_VERSION_COMPLETE "7.9.3"' "$sdk/include/opencascade/Standard_Version.hxx"; then
  echo 'Set OCCT_ROOT to OCCT 7.9.3.' >&2; exit 1
fi
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off -isystem "$sdk/include/opencascade" \
  "$project_dir/scripts/qualify-flange-lettering.cpp" -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" \
  -lTKDESTEP -lTKXSBase -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKGeomBase -lTKMath -lTKernel \
  -o "$out/qualify-flange-lettering" > "$out/compile.log" 2>&1 || { cat "$out/compile.log" >&2; exit 1; }
"$out/qualify-flange-lettering" "$1" "$2" "$out" > "$out/native.log" 2>&1 || { tail -12 "$out/native.log" >&2; exit 1; }
if [ "$1" = --frontier ]; then cat "$out/native.log"; else cat "$out/report.json"; fi
