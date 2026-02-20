#!/bin/bash
set -e

# Setup Directories
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/vars.env"

# Start Reth EL
echo "Starting Reth EL..."
RUST_LOG=info,reth_transaction_pool=debug $RETH_BIN node \
    --datadir "$DATA_DIR/reth" \
    --chain "$NETWORK_DIR/genesis.json" \
    --http \
    --http.api eth,net,web3,debug,trace,txpool \
    --http.port $RETH_HTTP_PORT \
    --builder.gaslimit 720000000000 \
    --builder.deadline 120 \
    --rpc.max-response-size 1000 \
    --txpool.gas-limit 720000000000 \
    --txpool.max-account-slots 3000 \
    --txpool.pending-max-size 512 \
    --txpool.queued-max-size 512 \
    --txpool.basefee-max-size 512 \
    --authrpc.jwtsecret "$NETWORK_DIR/jwt.hex" \
    --authrpc.addr 127.0.0.1 \
    --authrpc.port $RETH_AUTH_PORT \
    --discovery.port $RETH_P2P_PORT \
    --port $RETH_P2P_PORT \
    --log.file.directory "$LOG_DIR" \
    --color always
