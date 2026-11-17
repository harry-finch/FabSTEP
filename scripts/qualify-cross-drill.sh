#!/bin/sh
# Experimental native laboratory only; never called by the application.
set -eu
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
lab_dir="$project_dir/artifacts/cross-drill-lab"
mkdir -p "$lab_dir"
if [ -n "${OCCT_ROOT:-}" ]; then
  sdk_root=$OCCT_ROOT
else
  sdk_root=
  for candidate in "${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade" /opt/homebrew/opt/opencascade /usr/local/opt/opencascade /usr/local; do
    if [ -f "$candidate/include/opencascade/Standard_Version.hxx" ] && [ -d "$candidate/lib" ]; then
      sdk_root=$candidate
      break
    fi
  done
fi
if [ -z "$sdk_root" ]; then
  echo 'OCCT SDK absent; set OCCT_ROOT.' >&2
  exit 1
fi
sdk_version=$(sed -n 's/^#define OCC_VERSION_COMPLETE "\([^"]*\)"/\1/p' "$sdk_root/include/opencascade/Standard_Version.hxx")
if [ "$sdk_version" != 7.9.3 ]; then
  echo "This laboratory requires OCCT 7.9.3; found $sdk_version." >&2
  exit 1
fi
case "${1:-}" in
  ''|--inspect-only) ;;
  *) echo 'Usage: qualify-cross-drill.sh [--inspect-only]' >&2; exit 1 ;;
esac
certificate_flag=
if [ "${1:-}" != --inspect-only ]; then
  if [ ! -f "$project_dir/scripts/cross_drill_certificate.hxx" ]; then
    echo 'Certificate header absent; only --inspect-only is available.' >&2
    exit 1
  fi
  certificate_flag=-DCROSS_DRILL_WITH_CERTIFICATE
fi
cxx_bin=${CXX:-c++}
"$cxx_bin" -std=c++17 -O2 -fno-fast-math -ffp-contract=off $certificate_flag \
  -isystem "$sdk_root/include/opencascade" -I "$project_dir/scripts" \
  "$project_dir/scripts/qualify-cross-drill.cpp" -L "$sdk_root/lib" \
  -Wl,-rpath,"$sdk_root/lib" -lTKDESTEP -lTKXSBase -lTKPrim -lTKBO -lTKBool \
  -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKGeomBase -lTKGeomAlgo -lTKMath -lTKernel \
  -o "$lab_dir/qualify-cross-drill" > "$lab_dir/compile.log" 2>&1
lab_status=0
"$lab_dir/qualify-cross-drill" "$lab_dir" > "$lab_dir/native.log" 2>&1 || lab_status=$?
cat "$lab_dir/report.json"
exit "$lab_status"
