#!/bin/bash
set -e
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/vars.env"

# Run Setup
"$SCRIPT_DIR/setup.sh"

# Start Nodes
echo "Starting nodes..."

"$SCRIPT_DIR/start_reth.sh" > "$LOG_DIR/reth.log" 2>&1 &
RETH_PID=$!
echo "Reth started with PID $RETH_PID"

"$SCRIPT_DIR/start_beacon.sh" > "$LOG_DIR/beacon_node.log" 2>&1 &
BN_PID=$!
echo "Beacon Node started with PID $BN_PID"

# Give a small delay to ensure BN is up before VC tries to connect (optional but helpful)
sleep 2

"$SCRIPT_DIR/start_validator.sh" > "$LOG_DIR/validator_client.log" 2>&1 &
VC_PID=$!
echo "Validator Client started with PID $VC_PID"

trap "kill $RETH_PID $BN_PID $VC_PID" EXIT

echo "Testnet is running. Logs are in $LOG_DIR"
echo "Press Ctrl+C to stop."

wait $RETH_PID $BN_PID $VC_PID
