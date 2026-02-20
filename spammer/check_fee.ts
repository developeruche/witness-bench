import { createPublicClient, http, formatGwei } from "viem";
import { defineChain } from "viem";

const localTestnet = defineChain({
    id: 1234,
    name: "Local Testnet",
    network: "local-testnet",
    nativeCurrency: { decimals: 18, name: "Ether", symbol: "ETH" },
    rpcUrls: {
        default: { http: ["http://127.0.0.1:8546"] },
        public: { http: ["http://127.0.0.1:8546"] },
    },
});

const client = createPublicClient({
    chain: localTestnet,
    transport: http("http://127.0.0.1:8546")
});

async function main() {
    try {
        const block = await client.getBlock();
        console.log("Current Block:", block.number);
        console.log("Base Fee:", block.baseFeePerGas ? formatGwei(block.baseFeePerGas) + " gwei" : "undefined");
        console.log("Gas Limit:", block.gasLimit.toString());

        // Check pending block for base fee trend
        try {
            const pending = await client.getBlock({ blockTag: 'pending' });
            console.log("Pending Base Fee:", pending.baseFeePerGas ? formatGwei(pending.baseFeePerGas) + " gwei" : "undefined");
        } catch (e) {
            console.log("Could not fetch pending block");
        }

    } catch (error) {
        console.error("Error:", error);
    }
}

main();
