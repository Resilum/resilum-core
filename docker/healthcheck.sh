#!/bin/sh
set -eu

file="${RESILUM_HEALTH_FILE:-/config/state/health}"
limit="${RESILUM_HEALTH_MAX_AGE:-45}"

if ! mtime=$(stat -c %Y "$file" 2>/dev/null); then
    echo "no heartbeat at $file" >&2
    exit 1
fi

age=$(($(date +%s) - mtime))
if [ "$age" -gt "$limit" ]; then
    echo "heartbeat is ${age}s old, limit ${limit}s" >&2
    exit 1
fi
