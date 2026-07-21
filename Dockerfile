# syntax=docker/dockerfile:1
#
# Draft image for the resilumd daemon: a fully static musl binary in a scratch
# runtime. External transport binaries (yggdrasil, i2pd, tor) are COPYed in later
# alongside it.
#
# Multi-arch: build with buildx and switch the target triple per platform
# (x86_64-unknown-linux-musl / aarch64-unknown-linux-musl).

# ---- build: static musl binary ----
FROM rust:1.97-alpine3.24 AS build
RUN apk add --no-cache musl-dev=1.2.6-r2
WORKDIR /src
# Draft: copy everything. Optimize later with a Cargo.toml/Cargo.lock deps-cache
# layer so source-only changes don't re-fetch/rebuild dependencies.
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin resilumd \
 && strip target/x86_64-unknown-linux-musl/release/resilumd

# ---- runtime: nothing but the static binary ----
FROM scratch
COPY --from=build /src/target/x86_64-unknown-linux-musl/release/resilumd /resilumd
# Later: COPY yggdrasil / i2pd / tor binaries here.
ENTRYPOINT ["/resilumd"]
