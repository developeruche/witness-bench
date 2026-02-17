#!/bin/bash
set -e

# Setup Directories
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/vars.env"

# Start Lighthouse BN
echo "Starting Lighthouse Beacon Node..."
$LIGHTHOUSE_BIN bn \
    --datadir "$DATA_DIR/lighthouse" \
    --testnet-dir "$NETWORK_DIR" \
    --execution-endpoint http://127.0.0.1:$RETH_AUTH_PORT \
    --execution-jwt "$NETWORK_DIR/jwt.hex" \
    --http \
    --http-port $BN_HTTP_PORT \
    --port $BN_P2P_PORT \
    --discovery-port $BN_P2P_PORT \
    --enr-udp-port $BN_P2P_PORT \
    --enr-tcp-port $BN_P2P_PORT \
    --enr-address 127.0.0.1 \
    --debug-level info
