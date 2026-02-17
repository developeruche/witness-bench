# Local Reth + Lighthouse Testnet

This directory contains scripts to run a local testnet with Reth (EL) and Lighthouse (CL).

## Prerequisites
- Rust toolchain (`cargo`)
- `reth` binary built
- `lighthouse` binary built

## Scripts

### 1. `setup.sh`
**Run this first.**
- Cleans up old data `data/`, `logs/`, `network_config/`.
- Builds the `genesis_generator` tool.
- Generates genesis state (using a 2-pass process for hash consistency).
- Initializes Reth.
- Creates validator keys.

```bash
./setup.sh
```

### 2. Start Scripts
You can run these in separate terminals to monitor their output directly (remove redirection in scripts if you want stdout) or just to have control.

*   **`start_reth.sh`**: Starts the Execution Layer (Reth).
    *   Port: 30304 (P2P), 8546 (HTTP), 8552 (Auth)
*   **`start_beacon.sh`**: Starts the Consensus Layer (Lighthouse Beacon Node).
    *   Port: 9001 (P2P), 5053 (HTTP)
*   **`start_validator.sh`**: Starts the Validator Client.

### 3. `run.sh`
A convenience wrapper that runs `setup.sh` and then starts all three nodes in the background, tailing their logs to `logs/`.

```bash
./run.sh
```

## Configuration
Shared variables (ports, paths, binaries) are defined in `vars.env`.
You can override binary paths by setting `RETH_BIN` or `LIGHTHOUSE_BIN` env vars.

## Logs
Logs are stored in the `logs/` directory.
