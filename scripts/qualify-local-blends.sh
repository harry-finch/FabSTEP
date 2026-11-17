#!/bin/sh
# Fixed synthetic references only. No application code or external input.
set -eu
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
lab_dir="$project_dir/artifacts/local-blends-lab"
mkdir -p "$lab_dir"
sdk_root=${OCCT_ROOT:-}
if [ -z "$sdk_root" ]; then
  for candidate in "${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade" /usr/local/opt/opencascade /usr/local; do
    if [ -f "$candidate/include/opencascade/Standard_Version.hxx" ]; then sdk_root=$candidate; break; fi
  done
fi
version_header="$sdk_root/include/opencascade/Standard_Version.hxx"
if [ ! -f "$version_header" ] || ! grep -Fqx '#define OCC_VERSION_COMPLETE "7.9.3"' "$version_header"; then
  echo 'Set OCCT_ROOT to the OCCT 7.9.3 SDK.' >&2
  exit 1
fi
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off \
  -isystem "$sdk_root/include/opencascade" "$project_dir/scripts/qualify-local-blends.cpp" \
  -L "$sdk_root/lib" -Wl,-rpath,"$sdk_root/lib" -lTKDESTEP -lTKXSBase -lTKFillet -lTKPrim \
  -lTKBO -lTKBool -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKGeomBase -lTKGeomAlgo -lTKMath -lTKernel \
  -o "$lab_dir/qualify-local-blends" > "$lab_dir/compile.log" 2>&1
lab_status=0
"$lab_dir/qualify-local-blends" "$lab_dir" > "$lab_dir/native.log" 2>&1 || lab_status=$?
cat "$lab_dir/report.json"
exit "$lab_status"
