#!/bin/sh
# Give TraceLens raw-socket access so it can trace without running as root.
# Runs as root from the .deb/.rpm post-install hook. Failure is non-fatal: the app
# explains how to grant the capability manually.
set -e
BIN=/usr/bin/tracelens
if command -v setcap >/dev/null 2>&1 && [ -x "$BIN" ]; then
    setcap cap_net_raw+ep "$BIN" || echo "tracelens: could not set CAP_NET_RAW; run: sudo setcap cap_net_raw+ep $BIN" >&2
else
    echo "tracelens: setcap not found; run: sudo setcap cap_net_raw+ep $BIN" >&2
fi
exit 0
