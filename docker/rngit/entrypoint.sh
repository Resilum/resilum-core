#!/bin/sh
set -e

mkdir -p /config/rngit/reticulum /config/rngit/repos/public

if [ ! -f /config/rngit/config ] && [ -f /config/rngit/rngit.conf.example ]; then
    cp /config/rngit/rngit.conf.example /config/rngit/config
fi

if [ ! -f /config/rngit/reticulum/config ] && [ -f /config/rngit/reticulum.conf.example ]; then
    cp /config/rngit/reticulum.conf.example /config/rngit/reticulum/config
fi

exec rngit --config /config/rngit --rnsconfig /config/rngit/reticulum
