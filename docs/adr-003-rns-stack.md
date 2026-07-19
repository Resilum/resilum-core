# ADR-003 — Rust Reticulum stack: leviculum

Status: Accepted

## Context

`resilum-core` needs a Rust Reticulum stack to build on (the ADR-002 gate). Two
candidates were evaluated with the `eval/` harness (clone + build + inspect):

- **reticulum-rs** (Beechat) — MIT, simpler, oriented around their Kaonic radio
  hardware, **no C-ABI FFI crate in-repo**, less explicit interop, last
  commit ~3 weeks old.
- **leviculum** (Lew_Palm) — AGPL-3.0, layered workspace (`leviculum-core`
  no_std + `-std` + `-ffi` + embedded `-nrf`/`-micron` + interop tests + fuzz),
  last commit within a day, very active.

Verified on leviculum:

- **`leviculum-ffi` builds** and cbindgen emits `leviculum.h` — a ready,
  rich C-ABI: `lev_builder_new/identity/storage_path/add_tcp_client/
  add_tcp_server/add_udp/add_auto_interface`, plus **Resource** (`lev_send_resource`
  / `set_resource_strategy` / `accept` / `reject`) and request/response. This is
  most of the mobile FFI already done.
- **Drop-in wire-compat:** `lnsd`/`lncp`/`lnstatus` replace `rnsd`/`rncp`/
  `rnstatus`; tools + Sideband/Nomadnet work against `lnsd`; interop tested
  in CI.
- **musl-static, multi-arch** nightly `.deb` (amd64+arm64) — matches our
  Dockerfile approach.
- Explicitly targets a "future Android app".

## Decision

Build `resilum-core` on **leviculum** (`leviculum-core` + `leviculum-ffi`).

Consequence — **license**: leviculum is **AGPL-3.0**, a strong copyleft with a
network clause. Linking it makes the whole project (`resilum-core`, `resilumd`,
and the mobile app) AGPL-3.0: source must be made available to users, including
those interacting over the network. This is accepted — the project is open FOSS.
`resilum-core`'s `license` is set to `AGPL-3.0-only`; the mobile repo inherits
the same when it consumes the core.

## Known gaps to track

- **not yet production-ready** (both candidates are).
- **Multi-segment file SEND > 1 MB not implemented** in leviculum (receiving
  works) — Codeberg #27. Relevant to the long-term RNS-Resource OTA of APKs
  (10–50 MB); the near-term Obtainium+GitHub channel is unaffected. Track /
  contribute upstream.
- **Tor-onion dialing** remains Resilum's own logic on top, regardless of stack.

## Next

Wire `leviculum-ffi` / `leviculum-core` into `resilum-core`'s `Node` (bring up
interfaces from `Config`, drive discovery, open Links for egress).
