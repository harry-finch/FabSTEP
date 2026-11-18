#!/bin/sh
# Reproduce the open-shell experiment from the user's original STL.
set -eu
if [ "$#" -ne 2 ]; then
    echo 'Use FLANGE.stl OUTPUT_DIR (OCCT 7.9.3, Python with NumPy/SciPy)' >&2
    exit 2
fi
stl=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
mkdir -p "$2"
out=$(cd "$2" && pwd)
cd "$(dirname "$0")/.."
py=${PYTHON:-python3}
"$py" -c 'import numpy, scipy'
export FABSTEP_PROFILE_STL="$stl" FABSTEP_PROFILE_DIR="$out/source"
export FABSTEP_TORUS_LAB_DIR="$out/native"
FABSTEP_DUMP_PLANES=1 FABSTEP_DUMP_CONES=1 FABSTEP_DUMP_ANALYTIC_SEAMS=1 FABSTEP_DUMP_OWNERS=1 FABSTEP_DUMP_BORES=1 \
    cargo test -p fabstep-core --features large-stl --target-dir artifacts/r080/target \
    --release --offline --locked --test residual_planar_profile -- --ignored --exact residual_planar_profile
src=$FABSTEP_PROFILE_DIR
native=$FABSTEP_TORUS_LAB_DIR
asm="$out/assembly"
"$py" scripts/qualify-flange-assembly.py --long-flanks --check-negatives "$src" "$asm"
"$py" scripts/qualify-flange-ownership.py --long-flanks --check-negatives "$src" "$asm/patches.tsv" "$asm/counts.json" > "$out/ownership.json"
sh scripts/qualify-torus-sectors.sh --pierced-side "$asm/base.json" "$asm/flanks.json" "$asm/upper.json" "$asm/pierced.json"
"$py" scripts/qualify-flange-interface.py --check-negatives --map "$out/interface-map.json" "$asm/source-contour.json" \
    "$native/pierced-side/main-contour.json" > "$out/reference-interface.json"
sh scripts/qualify-torus-sectors.sh --long-flanks "$asm/base.json" "$asm/flanks.json" "$asm/upper.json" "$asm/pierced.json" "$asm/long.json"
"$py" scripts/qualify-flange-bores.py --check-negatives --shoulder-station 7 "$src" "$asm/base.json" "$out/bores"
"$py" scripts/qualify-flange-collar.py "$src" "$out/collar.txt" > "$out/collar.json"
sh scripts/qualify-flange-bores.sh "$native/long-flanks/long-flanks.step" "$out/bores/measurements.txt" "$out/bore-shell" "$out/collar.txt"
"$py" scripts/qualify-flange-shoulders.py "$src" "$out/interface-map.json" "$native/pierced-side/main-contour.json" "$out/shoulders.txt"
sh scripts/qualify-flange-shoulders.sh "$out/bore-shell/bores.step" "$out/shoulders.txt" "$out/shoulder-shell"
"$py" scripts/qualify-flange-lettering.py "$src" "$asm" "$out/lettering.txt" > "$out/lettering.json"
sh scripts/qualify-flange-lettering.sh "$out/shoulder-shell/shoulders.step" "$out/lettering.txt" "$out/final"
sh scripts/qualify-flange-lettering.sh --frontier "$out/final/lettering.step" "$out/frontier"
"$py" scripts/qualify-flange-remainder.py "$src" "$asm" "$out/bores" "$out/lettering.txt" \
    --source-frontier "$out/source-frontier.json" --native-frontier "$out/frontier/frontier.json" \
    --check-negatives > "$out/remainder.json"
"$py" scripts/qualify-flange-trim-neighbors.py "$src" "$asm/long.json" --source-rim "$out/source-frontier.json" \
    --native-rim "$out/frontier/frontier.json" --triangles "$out/trim-triangles.txt" > "$out/trim-neighbors.json"
"$py" scripts/qualify-flange-rail-ends.py "$src" "$out/source-frontier.json" "$out/frontier/frontier.json" "$out/rail-ends"
sh scripts/qualify-flange-rail-ends.sh "$out/final/lettering.step" "$out/rail-ends/rail-ends.txt" "$out/rail-shell"
"$py" scripts/qualify-flange-remainder.py "$src" "$asm" "$out/bores" "$out/lettering.txt" \
    --rail-ends "$out/rail-ends/rail-ends.json" --source-frontier "$out/final-source-frontier.json" \
    --native-frontier "$out/rail-shell/frontier.json" --check-negatives \
    --trim-neighbors "$out/trim-triangles.txt" "$out/trim-uses.txt" > "$out/rail-partition.json"
sh scripts/qualify-flange-trim-triangles.sh "$out/final/lettering.step" "$out/trim-triangles.txt" "$out/trim-triangles"
sh scripts/qualify-flange-trim-triangles.sh --join "$out/rail-shell/rail-ends.step" "$out/trim-triangles.txt" "$out/trim-uses.txt" "$out/trim-shell"
sh scripts/qualify-flange-lettering.sh --frontier "$out/trim-shell/trim-shell.step" "$out/trim-frontier"
"$py" scripts/qualify-flange-remainder.py "$src" "$asm" "$out/bores" "$out/lettering.txt" \
    --rail-ends "$out/rail-ends/rail-ends.json" --include-trims "$out/trim-triangles.txt" \
    --source-frontier "$out/trim-source-frontier.json" --native-frontier "$out/trim-frontier/frontier.json" \
    --check-negatives > "$out/trim-partition.json"
