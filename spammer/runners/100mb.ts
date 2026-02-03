import { SpamSequenceConfig } from "@developeruche/tx-spammer-sdk/dist/types";
import {
  CHAIN_ID,
  localTestnet,
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

export async function setup_100mb(): Promise<SpamSequenceConfig> {
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
  console.log("------    Setting up 100mb spam strategy");
  console.log("========================================");
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

  let strategy_100mb: SpamSequenceConfig = {
    rpcUrl: RPC_URL,
    chainId: CHAIN_ID,
    maxGasLimit: 60_000_000n,
    concurrency: 1,
    durationSeconds: 300,
    strategy: {
      mode: "mixed",
      strategies: [
        {
          percentage: 80,
          config: {
            mode: "transfer",
            amountPerTx: parseEther("0.0001"),
            depth: 1,
          },
        },
        {
          percentage: 10,
          config: {
            mode: "write",
            targetContract: spammerAddress,
            functionName: "spam",
            abi: SPAMMER_ABI as any,
            staticArgs: [],
          },
        },
        {
          percentage: 10,
          config: {
            mode: "deploy",
            bytecode: SPAMMER_BYTECODE,
            args: [],
          },
        },
      ],
    },
  };

  return strategy_100mb;
}
