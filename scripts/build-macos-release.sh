#!/bin/sh
# Build and verify a local Apple Silicon release candidate.
set -eu

cd "$(dirname "$0")/.."
if [ "$#" -gt 1 ]; then
    echo "Usage: $0 [new-output-directory]" >&2
    exit 2
fi

target=aarch64-apple-darwin
export MACOSX_DEPLOYMENT_TARGET=26.0
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/artifacts/macos-build}"
export HOMEBREW_PREFIX="${HOMEBREW_PREFIX:-/opt/homebrew}"
export OCCT_ROOT="${OCCT_ROOT:-$HOMEBREW_PREFIX/opt/opencascade}"

metadata=$(mktemp -t fabstep-release-metadata)
trap 'rm -f "$metadata"' EXIT
python3 scripts/package-macos.py preflight
npm --prefix ui run build
cargo rustc --offline --locked --release --target "$target" \
    --manifest-path crates/fabstep-desktop/Cargo.toml --features custom-protocol \
    -- -C link-arg=-Wl,-headerpad_max_install_names
cargo rustc --offline --locked --release --target "$target" -p fabstep-cli \
    -- -C link-arg=-Wl,-headerpad_max_install_names
cargo metadata --offline --locked --filter-platform "$target" --format-version 1 \
    --manifest-path crates/fabstep-desktop/Cargo.toml --features custom-protocol > "$metadata"

binaries="$CARGO_TARGET_DIR/$target/release"
python3 scripts/package-macos.py package "$binaries/fabstep-desktop" \
    --smoke-cli "$binaries/fabstep" --cargo-metadata "$metadata" "$@"
