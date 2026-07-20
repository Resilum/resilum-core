#!/bin/sh
# Format (autofix), lint (deny warnings), test. Run: ./check.sh
set -eu
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
