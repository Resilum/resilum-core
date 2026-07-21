# resilum-core

Rust core for a Resilum node — the **single source of truth** consumed by both
the server/container daemon and the mobile app. Transport, discovery and
egress-policy logic lives here; the frontends are thin shells over it.

## Status

Skeleton. The public API surface (node lifecycle, events) is defined and builds;
transport/discovery/policy logic is **not implemented yet**.

## Gate (read before implementing transports)

The core is meant to sit on top of a Rust Reticulum stack. Before porting real
logic, evaluate a Rust RNS implementation on real hardware and confirm it covers
the interfaces we need:

- Candidates: **Leviculum** and **reticulum-rs** (both wire-compatible with the
  reference, both marked *not yet production-ready* as of early 2026).
- Confirm coverage of the transports the project relies on (TCP/UDP/I2P/LoRa/
  packet-radio are present; Yggdrasil = TCP over the ygg netif; Tor-onion dialing
  is our own code and must be reimplemented regardless).

Until that gate passes, keep this a skeleton — do not wire a specific RNS crate
in blindly.

## Layout

```text
crates/
  resilum-core/   library: node API, transports, discovery, egress policy
  resilumd/       thin daemon binary — the Linux/server frontend (goes in Docker)
  resilum-ffi/    C-ABI (extern "C") wrapper for Dart FFI (consumed by mobile)
```

## Build

```sh
cargo build --workspace
cargo test -p resilum-core
```

`resilum-ffi` builds a `cdylib`/`staticlib`; for mobile it is cross-compiled per
target (`cargo-ndk` for Android, `cargo-lipo`/XCFramework for iOS) and loaded via
`dart:ffi`.

## Toolchain

Use **rustup** (not a distro package) — the mobile cross-compile targets are
added with `rustup target add <triple>`. Edition 2024.

## License

**AGPL-3.0-only.** The core builds on the leviculum Rust Reticulum stack
(AGPL-3.0), whose copyleft propagates to the whole project — core, daemon and
the mobile app. See `docs/adr-003-rns-stack.md`. Add the full license text:
`curl -o LICENSE https://www.gnu.org/licenses/agpl-3.0.txt`.
