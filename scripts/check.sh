#!/bin/sh
# Local checks, deliberately sequential: Cargo/doctests share build outputs.
set -eu
cd "$(dirname "$0")/.."
python3 -m unittest discover -s scripts/tests
occt_version_tests="$(mktemp "${TMPDIR:-/tmp}/fabstep-occt-version.XXXXXX")"
trap 'test ! -f "$occt_version_tests" || unlink "$occt_version_tests"' 0 HUP INT TERM
rustc --edition=2021 --test crates/fabstep-cad/build_support/occt_version.rs -o "$occt_version_tests"
"$occt_version_tests"
cargo fmt --all -- --check
cargo test --workspace --offline --locked
cargo clippy --workspace --all-targets --offline --locked -- -D warnings
npm --prefix ui run test
npm --prefix ui run check
npm --prefix ui run build
cargo fmt --manifest-path crates/fabstep-desktop/Cargo.toml -- --check
cargo check --manifest-path crates/fabstep-desktop/Cargo.toml --offline --locked
cargo test --manifest-path crates/fabstep-desktop/Cargo.toml --offline --locked
cargo clippy --manifest-path crates/fabstep-desktop/Cargo.toml --all-targets --offline --locked -- -D warnings
