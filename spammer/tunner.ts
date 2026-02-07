import * as fs from "fs";
import { SpamOrchestrator } from "@developeruche/tx-spammer-sdk/dist/SpamOrchestrator";
import { SpamSequenceConfig } from "@developeruche/tx-spammer-sdk/dist/types";
import {
  createPublicClient,
  createWalletClient,
  defineChain,
  http,
  parseEther,
} from "viem";
import { SPAMMER_ABI, SPAMMER_BYTECODE } from "./SpammerArtifact";
import { privateKeyToAccount } from "viem/accounts";
import {
  BATCH_TOUCHER_ARTIFACT_PATH,
  CHAIN_ID,
  OUTPUT_FILE,
  ROOT_PRIVATE_KEY,
  RPC_URL,
} from "./constants";

async function main() {
  let localTestnet = defineChain({
    id: 3151908,
    name: "Local Testnet",
    nativeCurrency: {
      name: "Ether",
      symbol: "ETH",
      decimals: 18,
    },
    rpcUrls: {
      default: { http: [RPC_URL] },
    },
  });

  const chain = { ...localTestnet, rpcUrls: { default: { http: [RPC_URL] } } };
  const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });
  const rootAccount = privateKeyToAccount(ROOT_PRIVATE_KEY as `0x${string}`);
  const rootClient = createWalletClient({
    account: rootAccount,
    chain,
    transport: http(RPC_URL),
  });

  const artifact = JSON.parse(
    fs.readFileSync(BATCH_TOUCHER_ARTIFACT_PATH, "utf-8"),
  );
  const hash = await rootClient.deployContract({
    abi: artifact.abi,
    bytecode: artifact.bytecode.object,
  });
  const receipt = await publicClient.waitForTransactionReceipt({ hash });
  let toucherAddress = receipt.contractAddress!;

  // 100mb = 8_000_000n, 300mb = 24_000_000n, 500mb = 38_000_000n
  const touchConfig: SpamSequenceConfig = {
    rpcUrl: RPC_URL,
    chainId: 31337,
    maxGasLimit: 38_000_000n,
    concurrency: 2,
    durationSeconds: 1000,
    strategy: {
      mode: "batch_toucher",
      inputFile: OUTPUT_FILE,
      batchSize: 50,
      toucherAddress: toucherAddress,
    },
  };

  const orchestrator = new SpamOrchestrator(
    touchConfig,
    ROOT_PRIVATE_KEY as `0x${string}`,
  );

  try {
    await orchestrator.setup(parseEther("1"));

    console.log("Waiting for next block to ensure packing...");
    await waitForNextBlock(publicClient);
    console.log("Block mined! Starting spam immediately...");

    let orchestratorOutput = await orchestrator.start();
    console.log(orchestratorOutput);

    // difference between gas used and gas limit
    // @ts-ignore
    console.log(
      `Gas difference: ${orchestratorOutput.totalGasUsed - orchestratorOutput.finalBlockGasUsed}`,
    );

    console.log("Fetching execution witness...");
    const witness = await publicClient.request({
      method: "debug_executionWitness" as any,
      // @ts-ignore
      params: [`0x${orchestratorOutput.blockNumber.toString(16)}`],
    });

    const witnessSize = JSON.stringify(witness).length;
    // write witness to file
    fs.writeFileSync("witness.json", JSON.stringify(witness));
    console.log(`Witness size: ${(witnessSize / 1024 / 1024).toFixed(2)} MB`);
  } catch (error) {
    console.error("Spam failed:", error);
  }

  // let blockNumber = 188n;
  // console.log("Fetching execution witness...");
  // const witness = await publicClient.request({
  //   method: "debug_executionWitness" as any,
  //   // @ts-ignore
  //   params: [`0x${blockNumber.toString(16)}`],
  // });

  // const witnessSize = JSON.stringify(witness).length;
  // // write witness to file
  // fs.writeFileSync("witness.json", JSON.stringify(witness));
  // console.log(`Witness size: ${(witnessSize / 1024 / 1024).toFixed(2)} MB`);
}

async function waitForNextBlock(client: any) {
  return new Promise<void>((resolve) => {
    const unwatch = client.watchBlockNumber({
      onBlockNumber: () => {
        unwatch();
        resolve();
      },
    });
  });
}

main();
