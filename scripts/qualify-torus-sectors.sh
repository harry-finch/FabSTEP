#!/bin/sh
# Independent OCCT laboratory. Optional open-shell replay from measured parameters.
set -eu
project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
lab_dir="$project_dir/artifacts/r085-native"
program=qualify-torus-sectors
report=report.json
case "${1:-}/$#" in
  /0) ;;
  --open-chain/2) program=qualify-open-torus-chain ;;
  --shared-tori/2|--belt/2|--base/2|--base-flanks/3|--upper-tubes/4|--side-planes/4|--pierced-side/5)
    program=qualify-shared-tori ;;
  *) echo 'Unexpected mode or argument count. See R-085 in docs/VALIDATION.md.' >&2; exit 2 ;;
esac
if [ "$#" -ge 2 ]; then
  mode=${1#--}
  lab_dir="$lab_dir/$mode"
  report="$mode-report.json"
fi
mkdir -p "$lab_dir"
input=/dev/null
if [ "$#" -ge 2 ]; then
  input="$lab_dir/measurements.txt"
  python3 - "$@" > "$input" <<'PYINPUT'
from pathlib import Path
import json
import sys

def load(name, limit=32768):
    path = Path(name)
    if path.stat().st_size > limit:
        raise ValueError("oversized local measurements")
    return json.loads(path.read_text())

def fields(s):
    return [*s["center"], *s["axis"], *s["start_direction"], s["major_radius_mm"],
            s["minor_radius_mm"], s["sweep_rad"], *s["meridian_bounds_rad"]]

def emit_tubes(tubes, count):
    if len(tubes) != count:
        raise ValueError("unexpected measured tube count")
    for tube in tubes:
        print(tube["sector"], tube["end"], *tube["extrusion_vector_world_mm"])

def emit(mode, belt_file, *files):
    data = load(belt_file)
    if mode == "--open-chain":
        print(*fields(data), *data["extrusions"][0]["extrusion_vector_world_mm"],
              *data["extrusions"][1]["extrusion_vector_world_mm"])
        return

    count, tube_count = (3, 0) if mode == "--shared-tori" else (7, 5)
    if len(data["sectors"]) != count:
        raise ValueError("unexpected measured sector count")
    print(count, tube_count)
    for sector in data["sectors"]:
        print(*fields(sector))
    emit_tubes(data["tubes"] if tube_count else [], tube_count)
    if mode in ("--shared-tori", "--belt"):
        return

    plane = data["bottom_plane"]
    holes = plane["holes"]
    if len(holes) != 5:
        raise ValueError("expected five holes in the measured plane")
    cones = all("far_station_mm" in h and "far_radius_mm" in h for h in holes)
    if not cones and (files or any("far_station_mm" in h or "far_radius_mm" in h for h in holes)):
        raise ValueError("incomplete set of exit rings")
    print(plane["station_mm"], int(cones), plane["area_mm2"] if cones else -1.)
    for h in holes:
        print(*h["center_yz_mm"], h["radius_mm"])
        if cones:
            print(h["far_station_mm"], h["far_radius_mm"])
    if mode == "--base":
        return

    flanks = load(files[0])["short_flanks"]
    if len(flanks) != 4:
        raise ValueError("expected four short flanks")
    for flank in flanks:
        print(flank["lower_sector"], flank["cylinder_radius_mm"], *fields(flank["upper_torus"]))
    if mode == "--base-flanks":
        return

    upper = load(files[1])
    emit_tubes(upper["upper_tubes"], 5)
    if mode == "--upper-tubes":
        return
    planes = upper["side_planes"]
    if len(planes) != 2:
        raise ValueError("expected two complete plane rectangles")
    for plane in planes:
        print(plane["sector"], plane["end"], plane["area_mm2"])
    if mode == "--side-planes":
        return

    side = load(files[2], 512 * 1024)["pierced_side"]
    holes = side["holes_uv_mm"]
    if len(holes) != 10 or sum(map(len, holes)) > 4096:
        raise ValueError("unexpected pierced side profile dimensions")
    print(side["sector"], side["end"], *side["origin_mm"], *side["along_direction"],
          side["length_mm"], side["height_mm"], side["area_mm2"])
    for polygon in holes:
        print(len(polygon))
        for u, v in polygon:
            print(u, v)

emit(*sys.argv[1:])
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
