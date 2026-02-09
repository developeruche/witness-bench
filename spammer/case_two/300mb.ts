import { createPublicClient, http } from "viem";
import { run_with_strategy, setup_300mb } from "../runners";
import { localTestnet, RPC_URL } from "../constants";

// For this case, we would like to find out how long it would that to process `engine_newPayload`
// but in this case the geth node have been modified to return a witness when `engine_newPayload`
// is called, We would call this `engine_newPayloadWithWitness` in the future.

async function run() {
  const publicClient = createPublicClient({
    chain: localTestnet,
    transport: http(RPC_URL),
  });

  let strategy_300mb = await setup_300mb();
  await run_with_strategy(strategy_300mb)!;

  // The result of this bench can now be obtained from the logs of the CL client. (lighthouse in this case) and the EL (geth in the case)
}

run();
