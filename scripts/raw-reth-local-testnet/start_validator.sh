#!/bin/bash
set -e

# Setup Directories
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/vars.env"

# Start Lighthouse VC
echo "Starting Lighthouse Validator Client..."
$LIGHTHOUSE_BIN vc \
    --datadir "$DATA_DIR/lighthouse_vc/node_1" \
    --testnet-dir "$NETWORK_DIR" \
    --beacon-nodes http://127.0.0.1:$BN_HTTP_PORT \
    --init-slashing-protection \
    --suggested-fee-recipient 0x123463a4B065722E99115D6c222f267d9cABb524 \
    --debug-level info
