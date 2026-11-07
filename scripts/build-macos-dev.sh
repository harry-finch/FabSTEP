#!/bin/sh
# Build local : l'application dépend du SDK OpenCASCADE installé sur ce Mac.
set -eu
cd "$(dirname "$0")/.."
npm --prefix ui run build
cargo build --manifest-path crates/fabstep-desktop/Cargo.toml --features custom-protocol --locked
app_dir="artifacts/FabSTEP.app/Contents"
mkdir -p "$app_dir/MacOS" "$app_dir/Resources"
cp crates/fabstep-desktop/target/debug/fabstep-desktop "$app_dir/MacOS/FabSTEP"
cp crates/fabstep-desktop/icons/icon.png "$app_dir/Resources/icon.png"
python3 - "$app_dir/Info.plist" <<'PY'
import json
import plistlib
from pathlib import Path
import sys

config = json.loads(Path("crates/fabstep-desktop/tauri.conf.json").read_text())
version = config["version"]
numeric = version.split("-", 1)[0]
Path(sys.argv[1]).write_bytes(plistlib.dumps({
    "CFBundleExecutable": "FabSTEP",
    "CFBundleIdentifier": config["identifier"] + ".dev",
    "CFBundleName": "FabSTEP",
    "CFBundleDisplayName": "FabSTEP",
    "CFBundlePackageType": "APPL",
    "CFBundleShortVersionString": numeric,
    "CFBundleVersion": numeric,
    "FabSTEPReleaseVersion": version,
    "LSMinimumSystemVersion": config["bundle"]["macOS"]["minimumSystemVersion"],
    "NSHighResolutionCapable": True,
}))
PY
printf 'Application locale prête : %s/artifacts/FabSTEP.app\n' "$PWD"
