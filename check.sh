#!/usr/bin/env bash
# Format (autofix), lint, typecheck, test, doc, supply-chain and config lint.
# A missing tool fails the run: a skipped step reads like a passed one.
#
# Run: ./check.sh [--docker]
set -euo pipefail
cd "$(dirname "$0")"

RUN_DOCKER=false
[ "${1:-}" = "--docker" ] && RUN_DOCKER=true

MAX_LINES=150
MAX_COMMENT_PCT=15

step() { printf '\n▶ %s\n' "$1"; }

# require <command> <how to install it>
require() {
    command -v "$1" >/dev/null 2>&1 && return 0
    printf '  ✗ %s is not installed\n    install: %s\n' "$1" "$2"
    exit 1
}

# Files git knows about plus new ones it does not ignore, so a file is linted
# before it is ever staged.
tracked_and_new() {
    git ls-files --cached --others --exclude-standard "$@"
}

step "rustfmt"
cargo fmt --all

step "taplo (TOML)"
require taplo 'cargo install taplo-cli --locked'
RUST_LOG=warn taplo fmt

step "file length (<= $MAX_LINES lines)"
cargo run --quiet -p xtask -- file-length "$MAX_LINES" crates

step "comment density (<= $MAX_COMMENT_PCT% of non-blank lines)"
# Comments inside code only: `///` and `//!` document the API, and no threshold
# separates a needed doc from prose — that is a review question. Counted off
# the parse tree, so a `//` inside a string literal is not a comment.
cargo run --quiet -p xtask -- comment-density "$MAX_COMMENT_PCT" crates

step "gitleaks (staged and history)"
require gitleaks 'https://github.com/gitleaks/gitleaks#installing'
# Rules live in .gitleaks.toml; the baseline holds what is already published,
# so only new findings fail. Staged first: that is what a commit is about to
# carry, and the tree can hold local edits that never become one.
gitleaks git --staged --no-banner --redact
gitleaks git --baseline-path .gitleaks-baseline.json --no-banner --redact

step "ast-grep (structural lints, lints/)"
require ast-grep 'cargo install ast-grep --locked  |  npm i -g @ast-grep/cli'
ast-grep scan

step "clippy (deny warnings)"
cargo clippy --workspace --all-targets -- -D warnings

step "test"
cargo test --workspace

step "doc (deny broken links)"
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet

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
foreign=$(grep -nE "(^|[|&;( ])(python3?|perl|ruby|node|deno)( |$)" check.sh \
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
    docker buildx build --load -f Dockerfile -t resilum-core:check .
else
    printf '  not requested (pass --docker to build)\n'
fi

printf '\n✓ all checks passed\n'
