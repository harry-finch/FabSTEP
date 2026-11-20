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
runuser -u fabstep-smoke -- xvfb-run -a -s "-screen 0 1600x1000x24" dbus-run-session -- bash -euo pipefail -c '
    export LIBGL_ALWAYS_SOFTWARE=1
    openbox >/tmp/fabstep-smoke-openbox.log 2>&1 &
    wm=$!
    fabstep-desktop &
    desktop=$!
    trap "kill $desktop $wm 2>/dev/null || true" EXIT
    for attempt in $(seq 1 45); do
        kill -0 "$desktop"
        window=$(xdotool search --onlyvisible --pid "$desktop" --name "^FabSTEP" | tail -1) || true
        if [ -n "$window" ]; then
            import -window "$window" /tmp/fabstep-smoke.png
            tesseract /tmp/fabstep-smoke.png /tmp/fabstep-smoke -l eng 2>/dev/null
            if grep -Eqi "Ouvrir STL|Open STL" /tmp/fabstep-smoke.txt; then
                echo "FabSTEP window and WebKit open-file control displayed."
                exit 0
            fi
        fi
        sleep 1
    done
    xwininfo -root -tree
    cat /tmp/fabstep-smoke.txt 2>/dev/null || true
    exit 1
'
