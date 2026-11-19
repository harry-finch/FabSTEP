#!/bin/sh
set -eu
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
out=${1:-artifacts/trim-edge-cache}
mkdir -p "$out"
"${CXX:-c++}" -std=c++17 -O2 -isystem "$sdk/include/opencascade" "$src/qualify-trim-edge-cache.cpp" \
  -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKMesh -lTKPrim -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKMath \
  -lTKernel -o "$out/qualify-trim-edge-cache"
"$out/qualify-trim-edge-cache"
