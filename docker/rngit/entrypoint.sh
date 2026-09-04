#!/bin/sh
set -e

mkdir -p /config/rngit/reticulum /config/rngit/repos/mirrors /config/rngit/state

if [ ! -f /config/rngit/config ] && [ -f /config/rngit/rngit.conf.example ]; then
    cp /config/rngit/rngit.conf.example /config/rngit/config
fi

if [ ! -f /config/rngit/reticulum/config ] && [ -f /config/rngit/reticulum.conf.example ]; then
    cp /config/rngit/reticulum.conf.example /config/rngit/reticulum/config
fi

# Expose the Repositories Destination to the resilum sidecar for mesh-side
# mirror discovery. `--print-identity` creates the identity on first run.
rngit --config /config/rngit --rnsconfig /config/rngit/reticulum --print-identity \
    | awk '/Repositories Destination/ { gsub(/[<>]/, "", $NF); print $NF }' \
    > /config/rngit/state/destination

say_what_we_serve() {
    ls -1 /config/rngit/repos/mirrors 2>/dev/null > /config/rngit/state/served
}

fetch_whatever_was_asked_for() {
    [ -f /config/rngit/state/wanted ] || return 0
    self=$(cat /config/rngit/state/destination)
    while read -r repo source; do
        [ -n "$repo" ] && [ -n "$source" ] || continue
        [ -e "/config/rngit/repos/mirrors/$repo" ] && continue
        rngit mirror --config /config/rngit --rnsconfig /config/rngit/reticulum \
            "$source" "rns://$self/mirrors/$repo" || true
    done < /config/rngit/state/wanted
}

keep_the_mirrors_we_were_asked_for() {
    while sleep "${RNGIT_MIRROR_EVERY:-300}"; do
        say_what_we_serve
        fetch_whatever_was_asked_for
    done
}

say_what_we_serve
keep_the_mirrors_we_were_asked_for &

exec rngit --config /config/rngit --rnsconfig /config/rngit/reticulum
