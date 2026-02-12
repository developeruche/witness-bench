# Witness size benchmark

This is an experimental repo to evaluate the change in latency of an ethereum node with increase in witness size. Comparing which is more a more efficent flow, introducing a new method to the `EngineAPI` called `engine_newPayloadWithWitness` or pairing these two methods together, `engine_newPayload` and `debug_executionProof`.

### The setup

We use kurtosis to spin up a local testnet with 1 validator and 1 execution client. We use `go-ethereum` as the execution client and `lighthouse` as the consensus client. We have to somehow create a block whose witness size is `100mb`, `300mb`, and `500mb` for this experiment. To get this don't we would need to bloat the `stateless-execution-witness`, the approach taken was to call so many very large contracts in a single block, calling these contract woould require the contract `runtime.code` to be included in the witness, 24kb+ adds up quickly.

This is a two step setup approach; 

1. Start up the local testnet using `run_base_line_geth.sh` or `run_with_new_payload_geth.sh` depending on which case we are testing. 
2. Deploy the large smart contracts using this script;

```bash
cd spammer
pnpm b:setup
```

> Note: Currently you would manually have to read the logs to get this insight, would implement a script to parse the log file and process the data. 

## Case One: `engine_newPayload` and `debug_executionProof`

This case would spam the node to create a block of a certain size (100mb, 300mb, 500mb), then would call `debug_executionProof` to get the proof for that block. 
- 100mb at block depth 0: `pnpm run c1:b0:100`
- 300mb at block depth 0: `pnpm run c1:b0:300`
- 500mb at block depth 0: `pnpm run c1:b0:500`
- 300mb at block depth 8: `pnpm run c1:b8:300`
- 300mb at block depth 32: `pnpm run c1:b32:300`
- 300mb at block depth 64: `pnpm run c1:b64:300`

## Case Two: `engine_newPayloadWithWitness`

This case would spam the node to create a block of a certain size (100mb, 300mb, 500mb), but during block production the `CL` would request for best block using `engine_newPayloadWithWitness` instead of `engine_newPayload`. 
- 100mb at block depth 0: `pnpm run c2:100`
- 300mb at block depth 0: `pnpm run c2:300`
- 500mb at block depth 0: `pnpm run c2:500`