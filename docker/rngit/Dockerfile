# syntax=docker/dockerfile:1.7
FROM alpine:3.21

RUN apk add --no-cache --no-scripts \
        tini=0.19.0-r3 \
        git=2.47.3-r0 \
        python3=3.12.14-r0 \
        py3-pip=24.3.1-r0 \
 && pip install --no-cache-dir --break-system-packages --root-user-action=ignore \
        rns==1.4.0 \
 && adduser -D -H -u 1000 rngit \
 && mkdir -p /config \
 && chown rngit:rngit /config

COPY --chmod=755 entrypoint.sh /entrypoint.sh
USER 1000:1000
ENTRYPOINT ["/sbin/tini", "--", "/entrypoint.sh"]
