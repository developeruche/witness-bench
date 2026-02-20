import { createPublicClient, http } from "viem";
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

const workerAddresses = [
    "0xA29dBA0DD62C89E256117c9327F48f002A92BD4B",
    "0xa2a8b8891D458b8A2729A1151F41Bd1da6d6366D",
    "0x30Ff19aa1156D7AC6Bda8413A46D84229354540C",
    "0x92A92f670e0F59a8aCb04b2bcf9b2736C0450165",
    "0xdeB1cda92b8844168141C26E6e2EB2c258deca8B"
] as const;

async function main() {
    try {
        const block = await client.getBlock();
        console.log("Current Block:", block.number);

        for (const addr of workerAddresses) {
            const nonce = await client.getTransactionCount({ address: addr });
            console.log(`Address: ${addr}, On-chain Nonce: ${nonce}`);
        }

        try {
            const poolContent = await client.request({ method: "txpool_content" as any });
            if (poolContent && poolContent.queued) {
                for (const addr of workerAddresses) {
                    if (poolContent.queued[addr]) {
                        const queuedNonces = Object.keys(poolContent.queued[addr]).map(Number).sort((a, b) => a - b);
                        console.log(`Address: ${addr}, Queued Nonces: ${queuedNonces.slice(0, 5).join(", ")}...`);
                    } else {
                        console.log(`Address: ${addr}, No queued txs`);
                    }
                }
            }
        } catch (e2) {
            console.log("txpool_content failed:", e2);
        }

    } catch (error) {
        console.error("Error:", error);
    }
}

main();
