# syntax=docker/dockerfile:1.7

FROM --platform=$BUILDPLATFORM ghcr.io/rust-cross/cargo-zigbuild:0.23.2 AS build
ARG TARGETPLATFORM
# PROFILE=quick trades an unoptimised binary for a far faster build when testing.
ARG PROFILE=release
WORKDIR /src
COPY rust-toolchain.toml /src/
RUN rustup toolchain install
COPY Cargo.toml Cargo.lock /src/
COPY crates/resilum-core/Cargo.toml /src/crates/resilum-core/
COPY crates/resilum-nostr/Cargo.toml /src/crates/resilum-nostr/
COPY crates/resilumd/Cargo.toml /src/crates/resilumd/
COPY crates/resilum-ffi/Cargo.toml /src/crates/resilum-ffi/
COPY crates/xtask/Cargo.toml /src/crates/xtask/
RUN mkdir -p crates/resilum-core/src crates/resilum-nostr/src crates/resilumd/src \
             crates/resilum-ffi/src crates/xtask/src \
 && touch crates/resilum-core/src/lib.rs crates/resilum-nostr/src/lib.rs \
          crates/resilum-ffi/src/lib.rs \
 && echo 'fn main() {}' > crates/resilumd/src/main.rs \
 && echo 'fn main() {}' > crates/xtask/src/main.rs \
 && CARGO_NET_GIT_FETCH_WITH_CLI=true cargo fetch

COPY . /src
RUN --mount=type=cache,target=/src/target \
    case "$TARGETPLATFORM" in \
      linux/amd64) target=x86_64-unknown-linux-musl ;; \
      linux/arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "no rust target mapped for $TARGETPLATFORM" >&2; exit 1 ;; \
    esac \
 && RUSTFLAGS="-C strip=symbols" CARGO_NET_GIT_FETCH_WITH_CLI=true \
    cargo zigbuild --profile "$PROFILE" --target "$target" -p resilumd \
 && cp "target/$target/$PROFILE/resilumd" /resilumd

FROM alpine:3.21 AS runtime
RUN apk add --no-cache --no-scripts \
        tini=0.19.0-r3 \
        yggdrasil=0.5.9-r5 \
        tor=0.4.9.11-r0 \
        i2pd=2.54.0-r0 \
        nftables=1.1.1-r0 \
        libcap-setcap=2.78-r0 \
 && setcap cap_net_admin+ep /usr/bin/yggdrasil \
 && setcap cap_net_admin+ep /usr/sbin/nft \
 && adduser -D -H -u 1000 resilum \
 && mkdir -p /config /var/run/yggdrasil \
 && chown resilum:resilum /config /var/run/yggdrasil
COPY --chmod=755 docker/entrypoint.sh docker/healthcheck.sh /
COPY deploy/config/*.example /usr/share/resilum/defaults/
COPY --from=build /resilumd /usr/local/bin/resilumd
RUN setcap cap_net_raw+ep /usr/local/bin/resilumd \
 && apk del --no-cache libcap-setcap
USER 1000:1000
HEALTHCHECK --interval=15s --timeout=5s --start-period=60s --retries=3 \
    CMD ["/healthcheck.sh"]
ENTRYPOINT ["/sbin/tini", "--", "/entrypoint.sh"]
