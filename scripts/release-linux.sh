#!/usr/bin/env bash
# Build the Ubuntu 26.04 amd64 desktop/CLI installer using the distro's OCCT 7.9 ABI.
set -euo pipefail
cd "$(dirname "$0")/.."
version=$(python3 -c 'import json; print(json.load(open("crates/fabstep-desktop/tauri.conf.json"))["version"])')
test "$(dpkg --print-architecture)" = amd64
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
stage="$tmp/package"
export CARGO_TARGET_DIR="$tmp/target"
export OCCT_ROOT="$tmp/occt-sdk"
mkdir -p "$OCCT_ROOT"
ln -s /usr/include "$OCCT_ROOT/include"
ln -s "/usr/lib/$(dpkg-architecture -qDEB_HOST_MULTIARCH)" "$OCCT_ROOT/lib"
npm --prefix ui ci
npm --prefix ui run build
cargo build --locked --release --manifest-path crates/fabstep-desktop/Cargo.toml --features custom-protocol
cargo build --locked --release -p fabstep-cli
install -Dm755 "$CARGO_TARGET_DIR/release/fabstep-desktop" "$stage/usr/bin/fabstep-desktop"
install -Dm755 "$CARGO_TARGET_DIR/release/fabstep" "$stage/usr/bin/fabstep"
strip --strip-unneeded "$stage/usr/bin/fabstep" "$stage/usr/bin/fabstep-desktop"
install -Dm644 ui/public/logo.svg "$stage/usr/share/icons/hicolor/scalable/apps/fr.fabapps.fabstep.svg"
mkdir -p "$stage/usr/share/applications"
cat > "$stage/usr/share/applications/fr.fabapps.fabstep.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=FabSTEP
Comment=Reconstruct STEP solids from STL and 3MF meshes
Comment[fr]=Reconstruire des solides STEP à partir de maillages STL et 3MF
Exec=fabstep-desktop
Icon=fr.fabapps.fabstep
Terminal=false
Categories=Graphics;Engineering;
DESKTOP
python3 scripts/release-notices.py "$stage/usr/share/doc/fabstep/licenses"
install -Dm644 LICENSE "$stage/usr/share/doc/fabstep/copyright"
mkdir -p "$stage/DEBIAN" "$tmp/debian"
cat > "$tmp/debian/control" <<'CONTROL'
Source: fabstep
Section: graphics
Priority: optional
Maintainer: FabApps <noreply@users.noreply.github.com>
Standards-Version: 4.6.2

Package: fabstep
Architecture: amd64
Description: FabSTEP desktop and command-line mesh reconstruction
CONTROL
# Discover runtime package names and minimum versions from the binaries' real ELF dependencies.
depends=$(cd "$tmp" && dpkg-shlibdeps -O "$stage/usr/bin/fabstep" "$stage/usr/bin/fabstep-desktop")
depends=${depends#shlibs:Depends=}
test -n "$depends"
cat > "$stage/DEBIAN/control" <<CONTROL
Package: fabstep
Version: $version
Architecture: amd64
Section: graphics
Priority: optional
Maintainer: FabApps <noreply@users.noreply.github.com>
Depends: $depends
Homepage: https://github.com/harry-finch/FabSTEP
Description: Reconstruct STEP solids from STL and 3MF meshes
 Native FabSTEP desktop application and fabstep command-line tool.
CONTROL
mkdir -p artifacts/linux
dpkg-deb --root-owner-group --build "$stage" "artifacts/linux/FabSTEP-${version}-ubuntu-26.04-amd64.deb"
