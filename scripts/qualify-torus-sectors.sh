#!/bin/sh
# Independent OCCT laboratory. Optional open-shell replay from measured parameters.
set -eu
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
lab_dir="$project_dir/artifacts/r085-native"
program=qualify-torus-sectors
report=report.json
case "$#" in
  0) ;;
  2)
    case "$1" in
      --open-chain)
        program=qualify-open-torus-chain
        lab_dir="$lab_dir/open-chain"
        report=open-chain-report.json ;;
      --belt)
        program=qualify-shared-tori
        lab_dir="$lab_dir/belt"
        report=belt-report.json ;;
      --shared-tori)
        program=qualify-shared-tori
        lab_dir="$lab_dir/shared-tori"
        report=shared-tori-report.json ;;
      *) exit 2 ;;
    esac
    ;;
  *) echo 'Use no arguments for synthetic solids, or --open-chain / --shared-tori / --belt MEASUREMENTS.json.' >&2; exit 2 ;;
esac
mkdir -p "$lab_dir"
input=/dev/null
if [ "$#" = 2 ]; then
  input="$lab_dir/measurements.txt"
  python3 - "$2" "$1" > "$input" <<'PYINPUT'
from pathlib import Path
import json
import sys
path = Path(sys.argv[1])
if path.stat().st_size > 32768:
    raise ValueError("oversize measured sector")
data = json.loads(path.read_text())
def fields(s):
    return [*s["center"], *s["axis"], *s["start_direction"], s["major_radius_mm"],
            s["minor_radius_mm"], s["sweep_rad"], *s["meridian_bounds_rad"]]
if sys.argv[2] in ("--shared-tori", "--belt"):
    belt = sys.argv[2] == "--belt"
    count, tube_count = (7, 5) if belt else (3, 0)
    tubes = data["tubes"] if belt else []
    if len(data["sectors"]) != count or len(tubes) != tube_count:
        raise ValueError("unexpected measured sector or tube count")
    print(count, tube_count)
    for sector in data["sectors"]:
        print(*fields(sector))
    for tube in tubes:
        print(tube["sector"], tube["end"], *tube["extrusion_vector_world_mm"])
else:
    print(*fields(data), *data["extrusions"][0]["extrusion_vector_world_mm"],
          *data["extrusions"][1]["extrusion_vector_world_mm"])
PYINPUT
fi
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
  -isystem "$sdk_root/include/opencascade" "$project_dir/scripts/$program.cpp" \
  -L "$sdk_root/lib" -Wl,-rpath,"$sdk_root/lib" -lTKDESTEP -lTKXSBase -lTKPrim \
  -lTKBO -lTKBool -lTKTopAlgo -lTKBRep -lTKG3d -lTKG2d -lTKGeomBase -lTKGeomAlgo -lTKMath -lTKernel \
  -o "$lab_dir/$program" > "$lab_dir/compile.log" 2>&1
if ! "$lab_dir/$program" "$lab_dir" < "$input" > "$lab_dir/native.log" 2>&1; then
  tail -n 8 "$lab_dir/native.log" >&2
  exit 1
fi
cat "$lab_dir/$report"
