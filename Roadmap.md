# Resilum Development Roadmap

Open work, grouped by area. An item is removed when it is done — the history
lives in the git log. Items are one line each; the detail belongs in the commit
that closes them.

## Security

- Egress links are accepted without authentication
- Covert server allocates a session before authentication, and the session table
  has no upper bound
- Covert discovery dials any IP a peer sends
- Yggdrasil discovery accepts any IPv6, including `::1`, ULA and link-local
- Yggdrasil private key is written without restricting permissions

## Correctness

- Reassembly buffer is not bounded by the window
- `SendBuffer::ack` loops on an ack value taken from the packet
- FakeDNS name map grows without eviction; the pool exhausts permanently
- FakeDNS decodes DNS labels lossily, so non-UTF-8 names lose bytes
- `.onion` and `.i2p` suffix checks are case-sensitive, DNS names are not

## Concurrency

- Blocking `fsync` runs inside the async task serving the announce bus
- Peer connect spawns detached tasks: an interface can attach after teardown
- The "already connected" check and the insert are split by an `.await`
- `active` is read outside the `handles` mutex
- `notify_waiters` during `announce_all` is lost rather than queued
- A `tokio::sync::Mutex` is held across a network `.await`, serialising i2p dials
- No backpressure anywhere on the byte path: unbounded channels, unbounded ARQ
  buffers

## Testing

- No per-test timeout: one hung test hangs the run
- `resilum-ffi` has 34 C ABI entry points and three smoke tests
- The tor, ygg and i2p transports and the shared SOCKS5 client have no tests
- Test parallelism is unbounded
- Parsers fed from the network have examples but no property or fuzz tests

## Operability

- Overlay daemons are started with `&` from the entrypoint; their death is
  neither logged nor visible to the healthcheck
- A failed SOCKS ingress bind is not logged and the task dies for good
- Concurrent sessions are logged without a correlation id
- The daemon config is not validated at startup: unknown keys and unknown values
  pass silently, and a malformed `socks:` value disables proxying fail-open
- Ten environment variables with no summary and no example

## Release

- The image builds only from a local checkout; there is no reproducible path
- No changelog, though the C ABI is consumed by a separate repository
- No tags, and the version has never moved from `0.0.0`
- No documented deploy or rollback path
- Nothing verifies the published tree
- The runtime image is Alpine 3.21, and its network daemons lag the build stage
- The status section of the README contradicts the code
