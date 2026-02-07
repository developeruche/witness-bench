import { SpamSequenceConfig } from "@developeruche/tx-spammer-sdk/dist/types";
import {
  BATCH_TOUCHER_ARTIFACT_PATH,
  CHAIN_ID,
  localTestnet,
  OUTPUT_FILE,
  ROOT_PRIVATE_KEY,
  RPC_URL,
} from "../constants";
import {
  Account,
  Chain,
  createPublicClient,
  createWalletClient,
  http,
  parseEther,
  PublicClient,
  Transport,
  WalletClient,
} from "viem";
import { SPAMMER_ABI, SPAMMER_BYTECODE } from "../SpammerArtifact";
import { privateKeyToAccount } from "viem/accounts";
import * as fs from "fs";

export async function setup_500mb(): Promise<SpamSequenceConfig> {
  const rootAccount = privateKeyToAccount(ROOT_PRIVATE_KEY as `0x${string}`);
  const rootClient = createWalletClient({
    account: rootAccount,
    chain: localTestnet,
    transport: http(RPC_URL),
  });
  const publicClient = createPublicClient({
    chain: localTestnet,
    transport: http(RPC_URL),
  });

  console.log("========================================");
  console.log("------    Setting up 500mb spam strategy");
  console.log("========================================");
  console.log("\n--- 0. Setup: Deploying Spammer.sol Target ---");
  const artifact = JSON.parse(
    fs.readFileSync(BATCH_TOUCHER_ARTIFACT_PATH, "utf-8"),
  );
  const hash = await rootClient.deployContract({
    abi: artifact.abi,
    bytecode: artifact.bytecode.object,
  });
  const receipt = await publicClient.waitForTransactionReceipt({ hash });
  let toucherAddress = receipt.contractAddress!;
  console.log("BatchToucher deployed at:", toucherAddress);

  let strategy_500mb: SpamSequenceConfig = {
    rpcUrl: RPC_URL,
    chainId: CHAIN_ID,
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

  return strategy_500mb;
}
