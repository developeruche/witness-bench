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

async function main() {
  const RPC_URL = process.env.RPC_URL || "http://127.0.0.1:55643";
  const ROOT_PRIVATE_KEY =
    process.env.PRIVATE_KEY ||
    "0x39725efee3fb28614de3bacaffe4cc4bd8c436257e2c8bb887c4b5c4be45e76d";

  // const RPC_URL = process.env.RPC_URL || 'http://127.0.0.1:8545';
  // const ROOT_PRIVATE_KEY =
  //     process.env.PRIVATE_KEY ||
  //     '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';

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

  console.log("\n--- 0. Setup: Deploying Spammer.sol Target ---");
  const deployHash = await rootClient.deployContract({
    abi: SPAMMER_ABI,
    bytecode: SPAMMER_BYTECODE,
  });
  console.log("Deploy tx sent:", deployHash);
  const receipt = await publicClient.waitForTransactionReceipt({
    hash: deployHash,
  });
  const spammerAddress = receipt.contractAddress!;
  console.log("Spammer deployed at:", spammerAddress);

  const mixedConfig: SpamSequenceConfig = {
    rpcUrl: RPC_URL,
    chainId: 3151908, // I think I should be changing this to 1
    maxGasLimit: 60_000_000n,
    concurrency: 1,
    durationSeconds: 300,
    strategy: {
      mode: "mixed",
      strategies: [
        // {
        //     percentage: 100,
        //     config: {
        //         mode: 'transfer',
        //         amountPerTx: parseEther('0.0001'),
        //         depth: 1,
        //     },
        // },
        {
          percentage: 100,
          config: {
            mode: "write",
            targetContract: spammerAddress,
            functionName: "spam",
            abi: SPAMMER_ABI as any,
            staticArgs: [],
          },
        },
        // {
        //     percentage: 50,
        //     config: {
        //         mode: 'deploy',
        //         bytecode: SPAMMER_BYTECODE,
        //         args: [],
        //     },
        // }
      ],
    },
  };

  const orchestrator = new SpamOrchestrator(
    mixedConfig,
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
