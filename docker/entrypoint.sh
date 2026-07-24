#!/bin/sh
set -e

seed_default() {
    src="/config/$1.example"
    dst="/config/$1"
    if [ -f "$src" ] && [ ! -f "$dst" ]; then
        cp "$src" "$dst"
    fi
}

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
        seed_default yggdrasil.conf
        if [ -f /config/yggdrasil.conf ]; then
            resilumd ygg-seed-keys /config/yggdrasil.conf || true
            yggdrasil -useconffile /config/yggdrasil.conf &
        fi
    fi
fi

if [ "${ENABLE_TOR:-1}" = "1" ] && command -v tor >/dev/null 2>&1; then
    seed_default torrc
    if [ -f /config/torrc ]; then
        mkdir -p /config/tor/data /config/tor/hidden_service
        chmod 700 /config/tor/hidden_service 2>/dev/null || true
        tor -f /config/torrc &
    fi
fi

if [ "${ENABLE_I2PD:-1}" = "1" ] && command -v i2pd >/dev/null 2>&1; then
    seed_default i2pd.conf
    seed_default i2pd-tunnels.conf
    if [ -f /config/i2pd.conf ]; then
        mkdir -p /config/i2p/data /config/i2p/keys /config/i2p/hidden_service
        i2pd --datadir=/config/i2p --conf=/config/i2pd.conf &
        resilumd i2pd-export-hostname \
            /config/i2p/keys/rns-server.dat \
            /config/i2p/hidden_service/hostname &
    fi
fi

exec resilumd /config/resilumd.yaml
