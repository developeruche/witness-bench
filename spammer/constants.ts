import { defineChain } from "viem";

export const RPC_URL = process.env.RPC_URL || "http://127.0.0.1:55835";
export const ROOT_PRIVATE_KEY =
  process.env.PRIVATE_KEY ||
  "0xbcdf20249abf0ed6d944c0288fad489e33f66b3960d9e6229c1cd214ed3bbe31";
export const CHAIN_ID = 3151908;
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
