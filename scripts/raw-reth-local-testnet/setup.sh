#!/bin/bash
set -e

# Setup Directories
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
source "$SCRIPT_DIR/vars.env"

# Check binaries
if [ ! -f "$RETH_BIN" ]; then
    echo "Reth binary not found at $RETH_BIN. Please build it first."
    exit 1
fi

if [ ! -f "$LIGHTHOUSE_BIN" ]; then
    echo "Lighthouse binary not found at $LIGHTHOUSE_BIN. Please build it first."
    exit 1
fi

# Cleanup
echo "Cleaning up old data..."
rm -rf "$NETWORK_DIR" "$LOG_DIR" "$DATA_DIR"
mkdir -p "$NETWORK_DIR" "$LOG_DIR" "$DATA_DIR"

# Build Genesis Generator
echo "Building Genesis Generator..."
cd "$GENESIS_GENERATOR_DIR"
cargo build --release
cd "$SCRIPT_DIR"

# Run Genesis Generator (Pass 1)
echo "Generating Genesis Files (Pass 1)..."
"$GENESIS_GENERATOR_DIR/target/release/genesis_generator" \
    --output-dir "$NETWORK_DIR" \
    --validator-count 1 \
    --chain-id 1234

# Initialize Reth
echo "Initializing Reth..."
# We need to capture the init output to get the genesis hash
INIT_OUTPUT=$($RETH_BIN init \
    --datadir "$DATA_DIR/reth" \
    --chain "$NETWORK_DIR/genesis.json" 2>&1)

echo "$INIT_OUTPUT"

# Extract Genesis Hash
# Remove potential color codes and extract hash
GENESIS_HASH=$(echo "$INIT_OUTPUT" | sed 's/\x1b\[[0-9;]*m//g' | grep -oE "hash=0x[a-f0-9]{64}" | cut -d= -f2)

if [ -z "$GENESIS_HASH" ]; then
    echo "Error: Could not extract genesis hash from Reth init output."
    # Print output for debugging
    echo "Init Output:"
    echo "$INIT_OUTPUT"
    exit 1
fi

echo "Extracted Genesis Hash: $GENESIS_HASH"

# Run Genesis Generator (Pass 2) to update genesis.ssz with correct hash
echo "Updating Genesis Files (Pass 2) with Execution Hash..."
"$GENESIS_GENERATOR_DIR/target/release/genesis_generator" \
    --output-dir "$NETWORK_DIR" \
    --validator-count 1 \
    --chain-id 1234 \
    --execution-hash "$GENESIS_HASH"

# Create Validators
echo "Creating Validators..."
$ROOT_DIR/baseline/lighthouse/target/release/lcli mnemonic-validators \
    --mnemonic-phrase "test test test test test test test test test test test junk" \
    --count 1 \
    --base-dir "$DATA_DIR/lighthouse_vc" \
    --node-count 1

echo "Setup complete. You can now run the start scripts."
