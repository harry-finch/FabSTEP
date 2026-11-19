#!/bin/sh
set -eu
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
out=${1:-artifacts/atlas-interference}
mkdir -p "$out"
"${CXX:-c++}" -std=c++17 -O2 -isystem "$sdk/include/opencascade" "$src/qualify-atlas-interference.cpp" \
  -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKBO -lTKGeomAlgo -lTKPrim -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKMath \
  -lTKernel -o "$out/qualify-atlas-interference"
"$out/qualify-atlas-interference"
