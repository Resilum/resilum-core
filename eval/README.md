# RNS stack evaluation

Decide which Rust Reticulum stack `resilum-core` sits on. This is the ADR-002
gate: **do not wire a stack into the core until one passes here.**

## Run

```
cd eval
docker build -t resilum-eval .
docker run --rm resilum-eval          # needs network to clone + fetch crates
```

(or `docker compose run --rm eval`). The script clones both candidates, builds
them, and lists what they produced. Then read the cloned sources under
`work/<name>/` to judge the API, and fill the table below.

## Candidates

- **reticulum-rs** — `github.com/BeechatNetworkSystemsLtd/Reticulum-rs`. Umbrella
  crate (`-core`, `-transport`, `-rpc`); TCP/UDP/I2P/LoRa/packet-radio; ships a
  C-ABI (`rns-embedded-ffi`) and `tcp_client`/`tcp_server` examples.
- **leviculum** — `codeberg.org/Lew_Palm/leviculum`. `no_std` `reticulum-core`;
  builds `lnsd`/`lncp`/`lns`; targets embedded + mobile.

Both are wire-compatible with the reference and both are marked *not yet
production-ready* (early 2026).

## Comparison (fill from the run + source reading)

| Criterion | reticulum-rs | leviculum |
|---|---|---|
| Builds cleanly (glibc) | | |
| Builds for musl / Android target | | |
| TCP / UDP | | |
| I2P (tunneled) | | |
| LoRa (RNode) | | |
| Packet radio | | |
| C-ABI FFI for Dart | rns-embedded-ffi | ? |
| `no_std` / embedded core | | reticulum-core |
| Resource (segmented file transfer, for OTA) | | ✓ (reported) |
| API fit for our `Node` model | | |
| Wire-compat verified vs a node | | |
| Activity / license | | |

## What "fit for our Node model" means

`resilum-core` exposes `Node { new / start / stop / send / poll_event }`. The
chosen stack must let us implement those on top of it: bring up interfaces from
config, drive discovery, and open Links for egress. A ready C-ABI (reticulum-rs)
is a plus for the mobile FFI, but the deciding factor is transport coverage +
whether the API maps cleanly onto our node lifecycle.

## Note

Tor-onion dialing (our `SocksTCPClientInterface`) is Resilum's own logic and is
reimplemented on top of whichever stack wins — it is not a selection criterion.
