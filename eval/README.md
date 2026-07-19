# RNS stack evaluation

Decide which Rust Reticulum stack `resilum-core` sits on. This is the ADR-002
gate. **Result: leviculum chosen — see `../docs/adr-003-rns-stack.md`.**

## Run

```
cd eval
docker build -t resilum-eval .
docker run --rm resilum-eval          # needs network to clone + fetch crates
```

(or `docker compose run --rm eval`). The script clones both candidates, builds
them, and lists what they produced.

## Candidates

- **reticulum-rs** — `github.com/BeechatNetworkSystemsLtd/Reticulum-rs`.
- **leviculum** — `codeberg.org/Lew_Palm/leviculum`.

Both are wire-compatible with the reference; both marked *not yet
production-ready* (early 2026).

## Comparison (filled from a run on 2026-07-19)

| Criterion | reticulum-rs | leviculum |
|---|---|---|
| License | **MIT** (permissive) | **AGPL-3.0** (copyleft, network clause) |
| Last commit | ~3 weeks | within a day (very active) |
| Builds here | not tested | ✅ `leviculum-ffi` built (23s) |
| C-ABI FFI for Dart | ❌ none in repo | ✅ `leviculum-ffi` + cbindgen `leviculum.h` |
| TCP client/server | ✓ | ✓ |
| UDP | example | ✓ (`lev_builder_add_udp`) |
| AutoInterface | — | ✓ (`lev_builder_add_auto_interface`) |
| LoRa | Kaonic (their HW) | ✓ (tested on real hardware) |
| Serial | ✓ | ✓ |
| Resource (for OTA) | ? | ✓ (`lev_send_resource` …) |
| `no_std` / embedded | yes | ✓ (`leviculum-core` + `-nrf`/`-micron`) |
| Wire-compat vs | implicit | ✓ drop-in `rnsd`/`rncp`/`rnstatus`, interop-tested |
| musl static / multi-arch | ? | ✓ ships musl `.deb` (amd64+arm64) |
| Mobile target | embeddable | ✓ explicit ("future Android app") |
| Quality signals | examples | fuzz + interop tests, nightly releases |

## Verdict

**leviculum.** It wins on nearly every axis for our needs (ready rich FFI with
cbindgen, Resource for OTA, drop-in interop, musl-static multi-arch,
explicit mobile target, most active). The one deciding tradeoff was its
**AGPL-3.0** license, which propagates to the whole project — accepted, since
Resilum is open FOSS.

reticulum-rs (MIT) would have been the fallback if a permissive license were
required, at the cost of writing the FFI ourselves and narrower transports.

## Known gap

leviculum's multi-segment file **send** > 1 MB is not implemented yet (receiving
works) — Codeberg #27. Matters for RNS-Resource OTA of large APKs; track upstream.

## Note

Tor-onion dialing (our `SocksTCPClientInterface`) is Resilum's own logic on top
of whichever stack wins — not a selection criterion.
