# syntax=docker/dockerfile:1.7

FROM --platform=$BUILDPLATFORM ghcr.io/rust-cross/cargo-zigbuild:0.23.2 AS build
ARG TARGETPLATFORM
WORKDIR /src
COPY . /src
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/src/target \
    case "$TARGETPLATFORM" in \
      linux/amd64) target=x86_64-unknown-linux-musl ;; \
      linux/arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "no rust target mapped for $TARGETPLATFORM" >&2; exit 1 ;; \
    esac \
 && rustup target add "$target" \
 && RUSTFLAGS="-C strip=symbols" cargo zigbuild --release --target "$target" -p resilumd \
 && cp "target/$target/release/resilumd" /resilumd

FROM alpine:3.21 AS runtime
RUN apk add --no-cache --no-scripts \
        tini=0.19.0-r3 \
        yggdrasil=0.5.9-r5 \
        tor=0.4.9.11-r0 \
        i2pd=2.54.0-r0 \
        libcap-setcap=2.78-r0 \
 && setcap cap_net_admin+ep /usr/bin/yggdrasil \
 && apk del libcap-setcap \
 && adduser -D -H -u 1000 resilum \
 && mkdir -p /config /var/run/yggdrasil \
 && chown resilum:resilum /config /var/run/yggdrasil
COPY --chmod=755 docker/entrypoint.sh docker/healthcheck.sh /
COPY deploy/config/*.example /usr/share/resilum/defaults/
COPY --from=build /resilumd /usr/local/bin/resilumd
USER resilum
HEALTHCHECK --interval=15s --timeout=5s --start-period=60s --retries=3 \
    CMD ["/healthcheck.sh"]
ENTRYPOINT ["/sbin/tini", "--", "/entrypoint.sh"]
