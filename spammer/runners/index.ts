import * as fs from "fs";
import {
  SpamResult,
  SpamSequenceConfig,
  SpamStrategyConfig,
} from "@developeruche/tx-spammer-sdk/dist/types";
import { ROOT_PRIVATE_KEY, RPC_URL } from "../constants";
import { foundry } from "viem/chains";
import { createPublicClient, createWalletClient, http, parseEther } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { SpamOrchestrator } from "@developeruche/tx-spammer-sdk/dist/SpamOrchestrator";

export * from "./100mb";
export * from "./300mb";
export * from "./500mb";

export async function run_with_strategy(
  strategy: SpamSequenceConfig,
): Promise<SpamResult | undefined> {
  const chain = { ...foundry, rpcUrls: { default: { http: [RPC_URL] } } };
  const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });
  const rootAccount = privateKeyToAccount(ROOT_PRIVATE_KEY as `0x${string}`);
  const rootClient = createWalletClient({
    account: rootAccount,
    chain,
    transport: http(RPC_URL),
  });

  const orchestrator = new SpamOrchestrator(
    strategy,
    ROOT_PRIVATE_KEY as `0x${string}`,
  );

  try {
    await orchestrator.setup(parseEther("1"));

    console.log("Waiting for next block to ensure packing...");
    await waitForNextBlock(publicClient);
    console.log("Block mined! Starting spam immediately...");

    let orchestratorOutput = await orchestrator.start();
    console.log(orchestratorOutput);

    return orchestratorOutput;
  } catch (error) {
    console.error("Spam failed:", error);
    return undefined;
  }
}

export async function waitForNextBlock(client: any) {
  return new Promise<void>((resolve) => {
    const unwatch = client.watchBlockNumber({
      onBlockNumber: () => {
        unwatch();
        resolve();
      },
    });
  });
}
