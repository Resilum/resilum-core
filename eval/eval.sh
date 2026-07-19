#!/bin/sh
# Evaluate candidate Rust Reticulum stacks side by side.
#
# Clones each repo, builds it, lists the binaries/examples it produced, and
# prints a summary. Fill the comparison table in README.md from the output and
# from reading each cloned source tree.
set -eu

WORK="${WORK:-/eval/work}"
mkdir -p "$WORK"

probe() {
    name="$1"; url="$2"; buildcmd="$3"
    echo "==================== $name ===================="
    dir="$WORK/$name"
    rm -rf "$dir"
    if ! git clone --depth 1 "$url" "$dir"; then
        echo "CLONE FAILED: $url"
        echo
        return 0
    fi
    cd "$dir"
    echo "--- HEAD ---"; git log -1 --oneline 2>/dev/null || true
    echo "--- crates in workspace ---"; find . -name Cargo.toml -not -path './target/*' | head -20
    echo "--- examples ---"; ls examples 2>/dev/null || echo "(none at top level)"
    echo "--- build: $buildcmd ---"
    if sh -c "$buildcmd"; then echo "BUILD OK"; else echo "BUILD FAILED"; fi
    echo "--- executables produced ---"
    find target -maxdepth 3 -type f -perm -u+x 2>/dev/null | grep -vE '\.(d|so|rlib)$' | head
    echo
}

# reticulum-rs (Beechat) — TCP/UDP/I2P/LoRa, C-ABI via rns-embedded-ffi,
# examples tcp_client/tcp_server.
probe "reticulum-rs" \
    "https://github.com/BeechatNetworkSystemsLtd/Reticulum-rs.git" \
    "cargo build --release"

# Leviculum (Lew_Palm) — no_std reticulum-core, builds lnsd/lncp/lns, targets
# embedded + mobile.
probe "leviculum" \
    "https://codeberg.org/Lew_Palm/leviculum.git" \
    "cargo build --release"

echo "Done. Inspect $WORK/<name>/ for sources, examples and the public API."
echo "Then fill the comparison table in eval/README.md."
