#!/usr/bin/env bash
# Run in a fresh Ubuntu container after apt installation, without the build SDKs.
set -euo pipefail
cd "$(dirname "$0")/.."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
test "$(dpkg-query -W -f='${Version}' fabstep)" = 1.0.0
fabstep convert fixtures/cube.stl --unit mm --mode auto --json -o "$tmp/cube.step" > "$tmp/cube.json"
fabstep convert fixtures/r117-cut-sphere.stl --unit mm --mode auto --json -o "$tmp/octant.step" > "$tmp/octant.json"
python3 - "$tmp" <<'PY'
import json
import math
import sys
from pathlib import Path

root = Path(sys.argv[1])
for name, faces, planes, spheres in (("cube", 6, 6, 0), ("octant", 4, 3, 1)):
    result = json.loads((root / f"{name}.json").read_text())
    cad = result["cad"]
    assert result["native_validated"] and cad["solids"] == 1
    assert (cad["faces"], cad["planar_faces"], cad["spherical_faces"]) == (faces, planes, spheres)
    assert (root / f"{name}.step").read_bytes().startswith(b"ISO-10303-21;")
    if name == "cube":
        assert math.isclose(cad["volume_mm3"], 1.0, rel_tol=1e-7)
PY
useradd --create-home fabstep-smoke
runuser -u fabstep-smoke -- dbus-run-session -- xvfb-run -a bash -ec '
    fabstep-desktop &
    desktop=$!
    trap "kill $desktop 2>/dev/null || true" EXIT
    timeout 45 xdotool search --sync --onlyvisible --name "^FabSTEP — STL / 3MF vers STEP$"
    kill -0 "$desktop"
'
