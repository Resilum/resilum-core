#!/usr/bin/env bash
# Run: ./checker.sh [--docker] [--check]
# A missing tool fails the run: a skipped step reads like a passed one.

set -euo pipefail
cd "$(dirname "$0")"

RUN_DOCKER=false
FORMAT_IN_PLACE=true
[ -n "${CI:-}" ] && FORMAT_IN_PLACE=false
for argument in "$@"; do
    case "$argument" in
        --docker) RUN_DOCKER=true ;;
        --check) FORMAT_IN_PLACE=false ;;
        *)
            printf '  ✗ unknown argument %s\n    run: ./checker.sh [--docker] [--check]\n' \
                "$argument"
            exit 1
            ;;
    esac
done

MAX_LINES=150
MAX_COMMENT_PCT=15

step() { printf '\n▶ %s\n' "$1"; }

require() {
    command -v "$1" >/dev/null 2>&1 && return 0
    printf '  ✗ %s is not installed\n    install: %s\n' "$1" "$2"
    exit 1
}

# So a file is linted before it is ever staged.
tracked_and_new() {
    git ls-files --cached --others --exclude-standard "$@"
}

step "rustfmt"
if $FORMAT_IN_PLACE; then cargo fmt --all; else cargo fmt --all --check; fi

step "taplo (TOML)"
require taplo 'cargo install taplo-cli --locked'
if $FORMAT_IN_PLACE; then RUST_LOG=warn taplo fmt; else RUST_LOG=warn taplo fmt --check; fi

step "file length (<= $MAX_LINES lines)"
cargo run --quiet -p xtask -- file-length "$MAX_LINES" crates

step "comment density (<= $MAX_COMMENT_PCT% of non-blank lines)"
# Code comments only: no threshold separates a needed `///` from prose, and a
# `//` inside a string literal is not a comment — both come off the parse tree.
cargo run --quiet -p xtask -- comment-density "$MAX_COMMENT_PCT" crates

step "gitleaks (staged and history)"
require gitleaks 'https://github.com/gitleaks/gitleaks#installing'
# The baseline holds what is already published, so only new findings fail.
# Staged first: that is what a commit is about to carry.
gitleaks git --staged --no-banner --redact
gitleaks git --baseline-path .gitleaks-baseline.json --no-banner --redact

step "ast-grep (structural lints, lints/)"
require ast-grep 'cargo install ast-grep --locked  |  npm i -g @ast-grep/cli'
ast-grep scan

step "clippy (deny warnings)"
cargo clippy --workspace --all-targets -- -D warnings

step "cross-compile the shared crate for Android (deny warnings)"
# Needs no NDK, as resilum-core pulls no C at this layer.
ANDROID_TARGET=aarch64-linux-android
require rustup 'https://rustup.rs'
rustup target list --installed | grep -qx "$ANDROID_TARGET" || {
    printf '  ✗ target %s is not installed\n    install: rustup target add %s\n' \
        "$ANDROID_TARGET" "$ANDROID_TARGET"
    exit 1
}
cargo clippy --quiet -p resilum-core --target "$ANDROID_TARGET" -- -D warnings

step "the daemon against the image's musl (deny warnings)"
require musl-gcc 'pacman -S musl  |  apt install musl-tools  |  apk add musl-dev'
MUSL_TARGET=x86_64-unknown-linux-musl
rustup target list --installed | grep -qx "$MUSL_TARGET" || {
    printf '  ✗ target %s is not installed\n    install: rustup target add %s\n' \
        "$MUSL_TARGET" "$MUSL_TARGET"
    exit 1
}
CC_x86_64_unknown_linux_musl=musl-gcc \
    cargo clippy --quiet -p resilumd --target "$MUSL_TARGET" -- -D warnings

step "test"
cargo test --workspace

step "doc (deny broken links)"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet

step "the local [patch], if any, is in effect"
# An ignored override means a run that reads as testing an override tested the
# pinned revision instead. A cargo diagnostic, so `-D warnings` misses it.
if [ -f .cargo/config.toml ]; then
    # Not `cargo metadata` — it resolves the same graph without reporting this.
    ignored=$(cargo check --workspace --all-targets 2>&1 >/dev/null |
        grep 'was not used in the crate graph' || true)
    if [ -n "$ignored" ]; then
        printf '%s\n' "$ignored"
        printf '  ✗ the override is ignored; this build used the pinned revision\n'
        exit 1
    fi
    printf '  in effect\n'
else
    printf '  none\n'
fi

step "Cargo.lock matches the pins"
# With the local [patch] active the lock loses the pinned revisions, so it is
# moved aside: the committed lock is the one a fresh clone resolves.
PATCH_CONFIG=.cargo/config.toml
PATCH_STASH=

restore_patch_config() {
    [ -n "$PATCH_STASH" ] || return 0
    mv "$PATCH_STASH" "$PATCH_CONFIG"
    PATCH_STASH=
}

if [ -f "$PATCH_CONFIG" ]; then
    PATCH_STASH=$(mktemp)
    mv "$PATCH_CONFIG" "$PATCH_STASH"
    trap restore_patch_config EXIT
    printf '  [patch] moved aside for this step\n'
fi
cargo metadata --format-version 1 --offline --quiet >/dev/null
cargo metadata --format-version 1 --offline --locked --quiet >/dev/null
restore_patch_config
trap - EXIT

step "scripts stay shell"
# A `python3 -c '...'` argument is just a string to the shell linter, so an
# embedded language passes every gate here while being linted by none.
foreign=$(grep -nE "(^|[|&;( ])(python3?|perl|ruby|node|deno)( |$)" checker.sh \
    | grep -v '^[0-9]*: *#' || true)
if [ -n "$foreign" ]; then
    printf '%s\n' "$foreign"
    printf '  ✗ the checker is shell and external tools; put logic in its own file\n'
    exit 1
fi

step "shellcheck"
require shellcheck 'https://github.com/koalaman/shellcheck#installing'
mapfile -t scripts < <(
    tracked_and_new -z | while IFS= read -r -d '' f; do
        case "$f" in
            *.sh) printf '%s\n' "$f" ;;
            *)
                [ -f "$f" ] || continue
                IFS= read -r shebang <"$f" 2>/dev/null || continue
                case "$shebang" in '#!'*sh | '#!'*sh\ *) printf '%s\n' "$f" ;; esac
                ;;
        esac
    done
)
[ "${#scripts[@]}" -gt 0 ] || { printf '  ✗ no scripts found to check\n'; exit 1; }
printf '  %s\n' "${scripts[@]}"
shellcheck "${scripts[@]}"

step "cargo-deny (advisories, bans, licenses, sources)"
require cargo-deny 'cargo install cargo-deny --locked'
# `-D warnings`, because cargo-deny exits 0 on them. `unmatched-source` fires
# only under a local [patch] that replaced the git dependency with a path.
cargo deny check advisories bans licenses sources -D warnings -A unmatched-source

step "hadolint (Dockerfiles)"
mapfile -t dockerfiles < <(tracked_and_new -- 'Dockerfile' '*/Dockerfile' '*.dockerfile')
[ "${#dockerfiles[@]}" -gt 0 ] || { printf '  ✗ no Dockerfile found to check\n'; exit 1; }
printf '  %s\n' "${dockerfiles[@]}"
if command -v hadolint >/dev/null 2>&1; then
    hadolint "${dockerfiles[@]}"
elif command -v docker >/dev/null 2>&1; then
    for f in "${dockerfiles[@]}"; do
        docker run --rm -i hadolint/hadolint:v2.14.0 hadolint - <"$f"
    done
else
    printf '  ✗ neither hadolint nor docker is installed\n'
    printf '    install: https://github.com/hadolint/hadolint#install (or any docker)\n'
    exit 1
fi

step "yamllint (strict: warnings fail)"
require yamllint 'pip install yamllint'
yamllint --strict .

step "markdownlint"
require markdownlint-cli2 'npm i -g markdownlint-cli2'
mapfile -t docs < <(tracked_and_new -- '*.md')
[ "${#docs[@]}" -gt 0 ] || { printf '  ✗ no markdown found to check\n'; exit 1; }
markdownlint-cli2 "${docs[@]}"

step "docker build"
if [ "$RUN_DOCKER" = true ]; then
    require docker 'https://docs.docker.com/engine/install/'
    read -ra also <<<"${DOCKER_BUILD_FLAGS:-}"
    docker buildx build --load "${also[@]}" -f Dockerfile -t resilum-core:check .
else
    printf '  not requested (pass --docker to build)\n'
fi

printf '\n✓ all checks passed\n'
