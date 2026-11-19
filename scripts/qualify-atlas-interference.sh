#!/bin/sh
set -eu
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
out=${1:-artifacts/atlas-interference}
if [ "$#" -gt 0 ]; then shift; fi
mkdir -p "$out"
build_and_run() {
  program=$1
  shift
  "${CXX:-c++}" -std=c++17 -O2 -isystem "$sdk/include/opencascade" "$src/$program.cpp" \
    -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKDESTEP -lTKXSBase -lTKOffset -lTKGeomBase -lTKBO -lTKGeomAlgo -lTKPrim -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKMath \
    -lTKernel -o "$out/$program"
  "$out/$program" "$@"
}
build_and_run qualify-atlas-interference "$@"
if [ "$#" -eq 0 ]; then
  build_and_run qualify-atlas-triangle
  build_and_run qualify-atlas-triangle-mesh
  build_and_run qualify-atlas-arc-law
fi
