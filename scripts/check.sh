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
# The large-source capability is built in its own target, never into the desktop.
cargo fmt --manifest-path crates/fabstep-worker/Cargo.toml -- --check
cargo test --manifest-path crates/fabstep-worker/Cargo.toml --offline --locked
cargo clippy --manifest-path crates/fabstep-worker/Cargo.toml --all-targets --offline --locked -- -D warnings
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib stl::tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib planar::tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib pipeline::reuse_tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib horizontal_patch_edge_tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib auto_proof_reuse_tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib rounded::torus_cone
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib rounded::rim_index_tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib revolved::adjacent_chain
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib rounded::zone::tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib partial::tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --test source_size_profiles --test analysis --test selection --test dense_planar_contours --test torus_cone_cylinder --test revolution_chains --test adjacent_chains --test local_trihedral
cargo test --target-dir crates/fabstep-worker/target -p fabstep-app --features large-stl --offline --locked --test source_size_profiles
cargo test --target-dir crates/fabstep-worker/target -p fabstep-app --features large-stl --offline --locked --lib rounded_face_limit_reports_the_count_separately_from_invalid_attribution
cargo test --target-dir crates/fabstep-worker/target -p fabstep-cad --features large-stl --offline --locked --lib size_profile_tests
cargo test --target-dir crates/fabstep-worker/target -p fabstep-cad --features large-stl --offline --locked --lib trihedral_octant
cargo test --target-dir crates/fabstep-worker/target -p fabstep-verify --features large-stl --offline --locked --lib
if [ "$(uname -s)" = Darwin ]; then
    cargo build --manifest-path crates/fabstep-worker/Cargo.toml --offline --locked
    FABSTEP_TEST_WORKER="$PWD/crates/fabstep-worker/target/debug/fabstep-worker" cargo test --manifest-path crates/fabstep-desktop/Cargo.toml --offline --locked real_worker_handoff -- --ignored
fi
