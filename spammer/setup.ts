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

  const blastConfig: SpamSequenceConfig = {
    rpcUrl: RPC_URL,
    chainId: CHAIN_ID,
    maxGasLimit: 9_000_000_000n,
    concurrency: 5,
    durationSeconds: 1000,
    strategy: {
      mode: "blast_large_contracts",
      contractCount: 12500,
      codeSize: 24 * 1024,
      outputFile: OUTPUT_FILE,
    },
  };

  for (let i = 0; i < 13; i++) {
    console.log(`Spam run ${i + 1} of 13`);
    const orchestrator = new SpamOrchestrator(
      blastConfig,
      ROOT_PRIVATE_KEY as `0x${string}`,
    );

    try {
      await orchestrator.setup(parseEther("1"));

      console.log("Waiting for next block to ensure packing...");
      await waitForNextBlock(publicClient);
      console.log("Block mined! Starting spam immediately...");

      let orchestratorOutput = await orchestrator.start();
      console.log(orchestratorOutput);
    } catch (error) {
      console.error("Spam failed:", error);
    }

    await waitForNextBlock(publicClient);
    console.log("Waiting for next block...");
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
