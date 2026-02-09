import { createPublicClient, http } from "viem";
import { run_with_strategy, setup_100mb, setup_300mb, waitForNextBlock } from "../runners";
import { localTestnet, RPC_URL } from "../constants";

// For this case, we would like to find out how long it would that to process `engine_newPayload`
// + `debug_executionWitness` for a block of depth 64, that is the latest block with a witness size
// of approx 300mb.

async function run() {
  const publicClient = createPublicClient({
    chain: localTestnet,
    transport: http(RPC_URL),
  });

  let strategy_300mb = await setup_300mb();
  let orchestratorOutput = await run_with_strategy(strategy_300mb)!;

  // at this poing the new payload api call have been dispatched by the CL
  // would like to call the `debug_executionWitness` api call and measure the time it takes to
  // receive the response. NOTE: for accurate results, this logs is been done on the CL clent.

  // For this is case we spam the next 64 blocks with 300mb of data each.
  // then would like to call the `debug_executionWitness` api call and measure the time it takes to
  // receive the response. NOTE: for accurate results, this logs is been done on the CL clent.

  for (let i = 0; i < 64; i++) {
    let strategy_100mb = await setup_100mb();
    await run_with_strategy(strategy_100mb)!;
    console.log(`Spammed delta-block ${i + 1}`);
  }

  console.log("Waiting for a block");
  await waitForNextBlock(publicClient);

  console.log("Fetching execution witness...");
  const start = performance.now();
  const witness = await publicClient.request({
    method: "debug_executionWitness" as any,
    // @ts-ignore
    params: [`0x${orchestratorOutput.blockNumber.toString(16)}`],
  });
  const end = performance.now();
  console.log(`Execution witness fetched in ${end - start} ms`);
  
  const witnessSize = JSON.stringify(witness).length;
  console.log(`Witness size: ${(witnessSize / 1024 / 1024).toFixed(2)} MB`);

  // The result of this bench can now be obtained from the logs of the CL client. (lighthouse in this case)
}

run();
