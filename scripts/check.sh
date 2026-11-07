#!/bin/sh
# Local checks, deliberately sequential: Cargo/doctests share build outputs.
set -eu
cd "$(dirname "$0")/.."
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
