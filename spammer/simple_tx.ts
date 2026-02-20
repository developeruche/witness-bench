import { createPublicClient, createWalletClient, http, parseEther, defineChain } from "viem";
import { privateKeyToAccount } from "viem/accounts";

const RPC_URL = "http://127.0.0.1:8546";
const ROOT_PRIVATE_KEY = "0x39725efee3fb28614de3bacaffe4cc4bd8c436257e2c8bb887c4b5c4be45e76d"; // Common dev key

const localTestnet = defineChain({
    id: 1234,
    name: "Local Testnet",
    network: "local-testnet",
    nativeCurrency: { decimals: 18, name: "Ether", symbol: "ETH" },
    rpcUrls: {
        default: { http: [RPC_URL] },
        public: { http: [RPC_URL] },
    },
});

async function main() {
    const publicClient = createPublicClient({
        chain: localTestnet,
        transport: http(RPC_URL)
    });
    const account = privateKeyToAccount(ROOT_PRIVATE_KEY as `0x${string}`);
    const walletClient = createWalletClient({
        account,
        chain: localTestnet,
        transport: http(RPC_URL)
    });

    console.log(`Sending tx from ${account.address}`);
    try {
        const hash = await walletClient.sendTransaction({
            to: "0x000000000000000000000000000000000000dEaD",
            value: parseEther("0.0001"),
        });
        console.log("Tx Sent! Hash:", hash);
        console.log("Waiting for receipt...");
        const receipt = await publicClient.waitForTransactionReceipt({
            hash,
            timeout: 30000 // 30s timeout
        });
        console.log("Tx Confirmed in block:", receipt.blockNumber);
    } catch (e) {
        console.error("Tx Failed:", e);
    }
}

main();
