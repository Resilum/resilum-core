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

exec rngit --config /config/rngit --rnsconfig /config/rngit/reticulum
