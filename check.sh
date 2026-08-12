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
too_long=$(find crates -name '*.rs' -exec awk -v max="$MAX_LINES" \
    'END { if (NR > max) printf "  %d\t%s\n", NR, FILENAME }' {} \; | sort -rn)
if [ -n "$too_long" ]; then
    printf '%s\n' "$too_long"
    printf '  ✗ file(s) over %s lines; split into a directory module\n' "$MAX_LINES"
    exit 1
fi

step "comment density (<= $MAX_COMMENT_PCT% of non-blank lines)"
# tokei rather than counting `//` lines: it parses the language, so a `//`
# inside a string literal is code and a doc block is not miscounted. Prose that
# outgrows this is usually restating what the code says.
require tokei 'cargo install tokei --locked'
require jq 'https://jqlang.github.io/jq/download/'
dense=$(tokei --output json crates \
    | jq -r --argjson max "$MAX_COMMENT_PCT" '
        .Rust.reports[]
        | select(.stats.comments + .stats.code > 0)
        | select(.stats.comments * 100 / (.stats.comments + .stats.code) > $max)
        | "  \(.stats.comments * 100 / (.stats.comments + .stats.code) | floor)%  \(.name)"')
if [ -n "$dense" ]; then
    printf '%s\n' "$dense"
    printf '  ✗ comment density over %s%%; cut what the code already says\n' "$MAX_COMMENT_PCT"
    exit 1
fi

step "index leaks (paths, language, private patterns)"
# The index, not the working tree: skip-worktree hides local edits only while
# that local flag survives. Patterns are categories, never literals — one
# spelling out what it hides would leak it; project-specific ones live in the
# untracked lints/private-patterns.txt.
# A home directory on any of the platforms someone might build this on:
# /home/<user>, /Users/<user>, C:\Users\<user>, /mnt/c/Users/<user>. `/root`
# is deliberately absent — it names no person and is a real path inside the
# container images.
home_dir='(/home|/Users|/mnt/[a-z]/Users|[A-Za-z]:[\\/]Users)[\\/][^\\/[:space:]"'"'"']+'
leak_patterns="$home_dir"'|[\x{0400}-\x{04FF}]'
if [ -f lints/private-patterns.txt ]; then
    while read -r pattern; do
        [ -n "$pattern" ] && leak_patterns="$leak_patterns|$pattern"
    done < <(grep -v '^#' lints/private-patterns.txt)
fi
# `:!` excludes this file, whose own pattern list would otherwise match.
if git grep --cached -nP "$leak_patterns" -- ':!check.sh'; then
    printf '  ✗ the index carries a local path, another language, or a private pattern\n'
    exit 1
fi

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

step "cargo-deny (advisories, licenses, sources)"
require cargo-deny 'cargo install cargo-deny --locked'
# `-D warnings`, because cargo-deny exits 0 on them. `unmatched-source` fires
# only under a local [patch] that replaced the git dependency with a path.
cargo deny check advisories licenses sources -D warnings -A unmatched-source

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
