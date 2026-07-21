#!/usr/bin/env bash
# Format (autofix), lint, typecheck, test, doc, supply-chain and config lint.
# Linters that are not installed are skipped with a note. Run: ./check.sh [--docker]
set -euo pipefail
cd "$(dirname "$0")"

RUN_DOCKER=false
[ "${1:-}" = "--docker" ] && RUN_DOCKER=true

have() { command -v "$1" >/dev/null 2>&1; }
step() { printf '\n▶ %s\n' "$1"; }
skip() { printf '  ⏭  skipped: %s\n' "$1"; }

step "rustfmt"
cargo fmt --all

step "clippy (deny warnings)"
cargo clippy --workspace --all-targets -- -D warnings

step "test"
cargo test --workspace

step "doc (deny broken links)"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet

step "shellcheck"
if have shellcheck; then
    shellcheck check.sh
else
    skip "shellcheck (pacman -S shellcheck)"
fi

step "cargo-deny (licenses + advisories)"
if have cargo-deny; then
    cargo deny check
else
    skip "cargo-deny (cargo install cargo-deny)"
fi

step "hadolint (Dockerfile)"
if have hadolint; then
    hadolint Dockerfile
elif have docker; then
    docker run --rm -i hadolint/hadolint:v2.14.0 hadolint - <Dockerfile
else
    skip "hadolint (cargo/pacman install, or docker)"
fi

step "yamllint"
if have yamllint; then
    yamllint crates/resilumd/resilumd.example.yaml
else
    skip "yamllint (pip install yamllint)"
fi

step "markdownlint"
if have markdownlint-cli2; then
    markdownlint-cli2 README.md docs/*.md
elif have markdownlint; then
    markdownlint README.md docs/*.md
else
    skip "markdownlint (npm i -g markdownlint-cli2)"
fi

if [ "$RUN_DOCKER" = true ]; then
    step "docker build"
    if have docker; then
        docker buildx build --load -f Dockerfile -t resilum-core:check .
    else
        skip "docker not available"
    fi
else
    step "docker build"
    skip "docker build (pass --docker to run)"
fi

printf '\n✓ all checks passed\n'
