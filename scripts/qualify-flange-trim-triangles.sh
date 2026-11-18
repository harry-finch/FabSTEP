#!/bin/sh
# Local OCCT laboratory. Compare isolated patches to the measured free STEP edges.
set -eu
case "${1:-}/$#" in
  --join/5) out=$5 ;;
  */3) out=$3 ;;
  *) echo 'Use QUALIFIED_STEP MANIFEST OUTPUT_DIR or --join QUALIFIED_STEP MANIFEST USES OUTPUT_DIR' >&2; exit 2 ;;
esac
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$out"
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
grep -Fqx '#define OCC_VERSION_COMPLETE "7.9.3"' "$sdk/include/opencascade/Standard_Version.hxx"
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off -isystem "$sdk/include/opencascade" \
  "$src/qualify-flange-trim-triangles.cpp" -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKDESTEP -lTKXSBase \
  -lTKOffset -lTKTopAlgo -lTKBRep -lTKG3d -lTKGeomBase -lTKG2d -lTKGeomAlgo -lTKMath -lTKernel \
  -o "$out/qualify-flange-trim-triangles"
if ! "$out/qualify-flange-trim-triangles" "$@" > "$out/native.log" 2>&1; then
  tail -8 "$out/native.log" >&2; exit 1
fi
cat "$out/report.json"
