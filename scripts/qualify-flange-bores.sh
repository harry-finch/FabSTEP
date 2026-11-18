#!/bin/sh
# Join the five measured bore chains and optional collar to a previously verified shell.
set -eu
{ [ "$#" = 3 ] || [ "$#" = 4 ]; } || { echo 'Use INPUT.step MEASUREMENTS.txt OUTPUT_DIR [COLLAR.txt]' >&2; exit 2; }
src_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$3"
sdk_root=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
grep -Fqx '#define OCC_VERSION_COMPLETE "7.9.3"' "$sdk_root/include/opencascade/Standard_Version.hxx"
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off \
  -isystem "$sdk_root/include/opencascade" "$src_dir/qualify-flange-bores.cpp" \
  -L "$sdk_root/lib" -Wl,-rpath,"$sdk_root/lib" -lTKDESTEP -lTKXSBase -lTKBO -lTKBool \
  -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKGeomBase -lTKGeomAlgo -lTKMath -lTKernel \
  -o "$3/qualify-flange-bores"
if ! "$3/qualify-flange-bores" "$@" > "$3/native.log" 2>&1; then
    tail -n 3 "$3/native.log" >&2
    exit 1
fi
tail -n 1 "$3/native.log" > "$3/report.json"
cat "$3/report.json"
