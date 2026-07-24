#!/bin/sh
set -e

mkdir -p /var/run/yggdrasil

ipv6_unavailable() {
    d=/proc/sys/net/ipv6/conf
    [ ! -e "$d/all/disable_ipv6" ] ||
        [ "$(cat "$d/all/disable_ipv6" 2>/dev/null)" = "1" ] ||
        [ "$(cat "$d/default/disable_ipv6" 2>/dev/null)" = "1" ]
}

if [ "${ENABLE_YGGDRASIL:-1}" = "1" ] && command -v yggdrasil >/dev/null 2>&1; then
    if ipv6_unavailable; then
        echo "[entrypoint] host IPv6 disabled; skipping yggdrasil (fix: sysctl -w net.ipv6.conf.all.disable_ipv6=0)"
    else
        if [ ! -f /config/yggdrasil.conf ] && [ -f /config/yggdrasil.conf.example ]; then
            cp /config/yggdrasil.conf.example /config/yggdrasil.conf
        fi
        if [ -f /config/yggdrasil.conf ]; then
            resilumd ygg-seed-keys /config/yggdrasil.conf || true
            yggdrasil -useconffile /config/yggdrasil.conf &
        fi
    fi
fi

exec resilumd /config/resilumd.yaml
