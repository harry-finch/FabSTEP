#!/bin/sh
# Synthetic R-090 laboratory; the application does not execute this script.
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
out=${1:-"$base/artifacts/r090-transfer"}
sdk=${OCCT_ROOT:-"${HOMEBREW_PREFIX:-/opt/homebrew}/opt/opencascade"}
python=${FREECAD_PYTHON:-/Applications/FreeCAD.app/Contents/Resources/bin/python}
export PYTHONPATH=${FREECAD_LIB:-"$(dirname -- "$python")/../lib"}${PYTHONPATH:+":$PYTHONPATH"}
if ! grep -Fqx '#define OCC_VERSION_COMPLETE "7.9.3"' "$sdk/include/opencascade/Standard_Version.hxx"; then
  echo 'Set OCCT_ROOT to the OCCT 7.9.3 SDK.' >&2
  exit 1
fi
mkdir -p "$out"
"$python" - "$base/scripts" "$out" <<'PY'
import json
import runpy
import sys
from pathlib import Path

scripts, out = map(Path, sys.argv[1:])
pocket = runpy.run_path(str(scripts / "qualify-unequal-pocket-freecad.py"))
seam = runpy.run_path(str(scripts / "qualify-unequal-pocket-curve.py"))["seam"]
chart = runpy.run_path(str(scripts / "qualify-unequal-pocket-traces.py"))
large, small = pocket["LARGE"], pocket["SMALL"]
pocket["source_shape"]().exportStep(str(out / "reference.step"))
curve, end = seam(large, small)
edge = curve.toShape()
edge.translate(pocket["CORNER"])
edge.exportStep(str(out / "quartic.step"))
proof = chart["qualify"](large, small, seam)
(out / "chart.json").write_text(json.dumps(proof, indent=2) + "\n")
n = proof["spans"]
with (out / "poles.dat").open("w") as f:
    print(n, file=f)
    for small_wall in (True, False):
        for p in chart["trace"](large, small, end, small_wall, n).getPoles():
            print(p.x, p.y, file=f)
PY
"${CXX:-c++}" -std=c++17 -O2 -fno-fast-math -ffp-contract=off \
  -isystem "$sdk/include/opencascade" "$base/scripts/qualify-unequal-pocket-transfer.cpp" \
  -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKDESTEP -lTKXSBase -lTKShHealing -lTKTopAlgo \
  -lTKBRep -lTKGeomBase -lTKGeomAlgo -lTKG3d -lTKG2d -lTKMath -lTKernel -o "$out/probe"
"$out/probe" "$out/reference.step" "$out/quartic.step" "$out/poles.dat" "$out/result.step"
"$python" - "$base/scripts" "$out" <<'PY'
import json
import runpy
import sys
from pathlib import Path

scripts, out = map(Path, sys.argv[1:])
pocket = runpy.run_path(str(scripts / "qualify-unequal-pocket-freecad.py"))
report = pocket["inspect"](out / "result.step", True, exact_transfer=True)
(out / "solid.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report, indent=2))
PY
