# syntax=docker/dockerfile:1.7

FROM rust:1.97-alpine3.24 AS build
RUN apk add --no-cache musl-dev=1.2.6-r2
WORKDIR /src
COPY . /src
RUN --mount=type=cache,target=/root/.cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --target x86_64-unknown-linux-musl -p resilumd \
 && strip target/x86_64-unknown-linux-musl/release/resilumd \
 && cp target/x86_64-unknown-linux-musl/release/resilumd /resilumd

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
COPY --chmod=755 docker/entrypoint.sh /entrypoint.sh
COPY --from=build /resilumd /usr/local/bin/resilumd
USER resilum
ENTRYPOINT ["/sbin/tini", "--", "/entrypoint.sh"]
