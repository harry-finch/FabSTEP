#!/bin/sh
# Reproducible private checkpoint or public independent source-only laboratory.
set -eu
case "$1/$#" in
  --checkpoint/3|--source-stl/3) mode=$1; input=$2; out=$3 ;;
  --public/2) mode=$1; out=$2 ;;
  *) echo 'Use --checkpoint CHECKPOINT OUTPUT_DIR, --source-stl INPUT.stl OUTPUT_DIR, or --public OUTPUT_DIR' >&2; exit 2 ;;
esac
src=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
mkdir -p "$out"
py=${PYTHON:-python3}
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
grep -Fqx '#define OCC_VERSION_COMPLETE "7.9.3"' "$sdk/include/opencascade/Standard_Version.hxx"
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off -isystem "$sdk/include/opencascade" \
    "$src/qualify-flange-residual.cpp" -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKDESTEP -lTKXSBase \
    -lTKMesh -lTKOffset -lTKTopAlgo -lTKBRep -lTKG3d -lTKGeomBase -lTKG2d -lTKGeomAlgo -lTKMath -lTKernel \
    -o "$out/qualify-flange-residual"
if [ "$mode" = --checkpoint ]; then
    "$py" "$src/qualify-flange-residual.py" "$input" "$out/witness.txt" >"$out/witness.json"
    open_shell="$input/trim-shell/trim-shell.step"
    graph_source=$input
else
    if [ "$mode" = --public ]; then
        input="$out/notched-toroidal-ramp.stl"
        "$py" "$src/qualify-flange-public.py" ramp "$input"
    fi
    "$py" "$src/qualify-flange-public.py" witness "$input" "$out/witness.txt"
    open_shell=--source-only
    graph_source="$out/witness.txt"
fi
"$py" "$src/qualify-flange-residual-seeds.py" "$out/witness.txt" >"$out/seeds.json"
"$py" "$src/qualify-flange-residual-tori.py" "$out/witness.txt" "$out/seeds.json" "$out/charts.txt" >"$out/charts.json"
# --assess records an over-budget native mesh as a laboratory measurement.
# All geometric checks still run. No file is sent to the application.
"$out/qualify-flange-residual" --assess "$open_shell" "$out/witness.txt" "$out/candidate.step" "$out/charts.txt" >"$out/native.log" 2>&1
"$out/qualify-flange-residual" --inspect "$out/candidate.step" "$out/graph.txt" .005 >"$out/inspect.log" 2>&1
"$py" "$src/qualify-flange-residual-graph.py" "$graph_source" "$out/graph.txt" "$out/owners.bin" --charts "$out/charts.txt" >"$out/ownership.json"
cat "$out/charts.json" "$out/ownership.json"
