import { defineChain } from "viem";

export const RPC_URL = "http://127.0.0.1:8546";
export const ROOT_PRIVATE_KEY =
  "0x39725efee3fb28614de3bacaffe4cc4bd8c436257e2c8bb887c4b5c4be45e76d";
export const CHAIN_ID = 1234;

// export const RPC_URL = process.env.RPC_URL || "http://127.0.0.1:8545";
// export const ROOT_PRIVATE_KEY =
//   process.env.PRIVATE_KEY ||
//   "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
// export const CHAIN_ID = 31337;

export const localTestnet = defineChain({
  id: CHAIN_ID,
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

export const BATCH_TOUCHER_ARTIFACT_PATH =
  "/Users/gregg/Documents/work/ethereum/witness-bench/spammer/BatchToucher.json";
export const OUTPUT_FILE =
  "/Users/gregg/Documents/work/ethereum/witness-bench/spammer/deployed_addresses.txt";
