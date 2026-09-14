#!/usr/bin/env bash
# Run: ./checker.sh [--docker | --images-only] [--check]
# A missing tool fails the run: a skipped step reads like a passed one.

set -euo pipefail
cd "$(dirname "$0")"

USAGE='  run: ./checker.sh [--docker | --images-only] [--check]\n'
RUN_DOCKER=false
IMAGES_ONLY=false
FORMAT_IN_PLACE=true
[ -n "${CI:-}" ] && FORMAT_IN_PLACE=false
for argument in "$@"; do
    case "$argument" in
        --docker) RUN_DOCKER=true ;;
        --images-only)
            RUN_DOCKER=true
            IMAGES_ONLY=true
            ;;
        --check) FORMAT_IN_PLACE=false ;;
        *)
            printf "  ✗ unknown argument %s\n$USAGE" "$argument"
            exit 1
            ;;
    esac
done

MAX_LINES=150
MAX_COMMENT_PCT=15
GROUPED_STD_OTHERS_OURS='group_imports=StdExternalCrate,imports_granularity=Module'
FROM_MISE='mise install  (versions come from mise.toml)'

step() { printf '\n▶ %s\n' "$1"; }

require() {
    command -v "$1" >/dev/null 2>&1 && return 0
    printf '  ✗ %s is not installed\n    install: %s\n' "$1" "$2"
    exit 1
}

require mise 'https://mise.jdx.dev/getting-started.html'
what_mise_pins=$(mise env -s bash) || {
    printf '  ✗ mise will not read mise.toml\n    run: mise trust\n'
    exit 1
}
eval "$what_mise_pins"

build_the_images() {
    require docker 'https://docs.docker.com/engine/install/'
    read -ra also <<<"${DOCKER_BUILD_FLAGS:-}"
    docker buildx build --load "${also[@]}" -f Dockerfile -t resilum-core:check .
    docker buildx build --load "${also[@]}" -t resilum-core-rngit:check docker/rngit
}

if $IMAGES_ONLY; then
    step "docker build"
    build_the_images
    printf '\n✓ the images build\n'
    exit 0
fi

# So a file is linted before it is ever staged.
tracked_and_new() {
    git ls-files --cached --others --exclude-standard "$@"
}

step "rustfmt"
if $FORMAT_IN_PLACE; then cargo fmt --all; else cargo fmt --all --check; fi

step "import grouping (rustfmt $RUSTFMT_THAT_GROUPS_IMPORTS)"
rustup toolchain list | grep -q "^$RUSTFMT_THAT_GROUPS_IMPORTS" ||
    {
        printf '  ✗ %s is not installed\n    install: rustup toolchain install %s --profile minimal --component rustfmt\n' \
            "$RUSTFMT_THAT_GROUPS_IMPORTS" "$RUSTFMT_THAT_GROUPS_IMPORTS"
        exit 1
    }
grouped=(fmt --all)
$FORMAT_IN_PLACE || grouped+=(--check)
cargo "+$RUSTFMT_THAT_GROUPS_IMPORTS" "${grouped[@]}" -- --config "$GROUPED_STD_OTHERS_OURS"

step "taplo (TOML)"
require taplo "$FROM_MISE"
if $FORMAT_IN_PLACE; then RUST_LOG=warn taplo fmt; else RUST_LOG=warn taplo fmt --check; fi

step "file length (<= $MAX_LINES lines)"
cargo run --quiet -p xtask -- file-length "$MAX_LINES" crates

step "comment density (<= $MAX_COMMENT_PCT% of non-blank lines)"
# Code comments only: no threshold separates a needed `///` from prose, and a
# `//` inside a string literal is not a comment — both come off the parse tree.
cargo run --quiet -p xtask -- comment-density "$MAX_COMMENT_PCT" crates

step "gitleaks (staged and history)"
require gitleaks "$FROM_MISE"
# The baseline holds what is already published, so only new findings fail.
# Staged first: that is what a commit is about to carry.
gitleaks git --staged --no-banner --redact
gitleaks git --baseline-path .gitleaks-baseline.json --no-banner --redact

step "ast-grep (structural lints, lints/)"
require ast-grep "$FROM_MISE"
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
require cargo-nextest "$FROM_MISE"
cargo nextest run --workspace

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
require shellcheck "$FROM_MISE"
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
shellcheck --external-sources "${scripts[@]}"

step "cargo-deny (advisories, bans, licenses, sources)"
require cargo-deny "$FROM_MISE"
# `-D warnings`, because cargo-deny exits 0 on them. `unmatched-source` fires
# only under a local [patch] that replaced the git dependency with a path.
cargo deny check advisories bans licenses sources -D warnings -A unmatched-source

step "Cargo.lock matches the pins (last, so no cargo step rewrites it after)"
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

step "hadolint (Dockerfiles)"
require hadolint "$FROM_MISE"
mapfile -t dockerfiles < <(tracked_and_new -- 'Dockerfile' '*/Dockerfile' '*.dockerfile')
[ "${#dockerfiles[@]}" -gt 0 ] || { printf '  ✗ no Dockerfile found to check\n'; exit 1; }
printf '  %s\n' "${dockerfiles[@]}"
hadolint "${dockerfiles[@]}"

step "yamllint (strict: warnings fail)"
require yamllint "$FROM_MISE"
yamllint --strict .

step "markdownlint"
require markdownlint-cli2 "$FROM_MISE"
mapfile -t docs < <(tracked_and_new -- '*.md')
[ "${#docs[@]}" -gt 0 ] || { printf '  ✗ no markdown found to check\n'; exit 1; }
markdownlint-cli2 "${docs[@]}"

step "docker build"
if [ "$RUN_DOCKER" = true ]; then
    build_the_images
else
    printf '  not requested (pass --docker to build)\n'
fi

printf '\n✓ all checks passed\n'
