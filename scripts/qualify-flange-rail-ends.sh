#!/bin/sh
# Join both measured planar rail ends to the reimported laboratory shell.
set -eu
[ "$#" = 3 ] || { echo 'Use INPUT.step RAIL-ENDS.txt OUTPUT_DIR' >&2; exit 2; }
src_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$3"
sdk_root=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
grep -Fqx '#define OCC_VERSION_COMPLETE "7.9.3"' "$sdk_root/include/opencascade/Standard_Version.hxx"
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off \
  -isystem "$sdk_root/include/opencascade" "$src_dir/qualify-flange-rail-ends.cpp" \
  -L "$sdk_root/lib" -Wl,-rpath,"$sdk_root/lib" -lTKDESTEP -lTKXSBase -lTKBO -lTKBool \
  -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKGeomBase -lTKGeomAlgo -lTKMath -lTKernel \
  -o "$3/qualify-flange-rail-ends"
if ! "$3/qualify-flange-rail-ends" "$@" > "$3/native.log" 2>&1; then
    tail -n 3 "$3/native.log" >&2
    exit 1
fi
cat "$3/report.json"
