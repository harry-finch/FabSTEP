#!/bin/sh
# Reimport and sample an independently built, source-certified STEP checkpoint.
set -eu
test "$#" -ge 1 && test "$#" -le 2 || { echo 'Use --witness or CERTIFIED_CHECKPOINT.step [OUTPUT_DIR]' >&2; exit 2; }
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cad="$src/../crates/fabstep-cad/src"
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
out=${2:-artifacts/trim-face-mesh}
mkdir -p "$out"
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off -isystem "$sdk/include/opencascade" \
  -I "$cad" "$src/qualify-trim-face-mesh.cpp" "$cad/cross_drill_verify.cpp" \
  -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKDESTEP -lTKXSBase -lTKMesh -lTKPrim -lTKFillet -lTKBO -lTKOffset \
  -lTKTopAlgo -lTKBRep -lTKGeomBase -lTKGeomAlgo -lTKG3d -lTKG2d -lTKMath -lTKernel -o "$out/qualify-trim-face-mesh"
"$out/qualify-trim-face-mesh" "$1"
