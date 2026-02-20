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

async function main() {
    try {
        // Request the txpool content
        const poolContent = await client.request({ method: "txpool_content" as any }) as any;

        const allTransactions: any[] = [];

        // Collect all pending transactions
        if (poolContent?.pending) {
            for (const address of Object.keys(poolContent.pending)) {
                for (const nonce of Object.keys(poolContent.pending[address])) {
                    allTransactions.push(poolContent.pending[address][nonce]);
                }
            }
        }

        // Collect all queued transactions
        if (poolContent?.queued) {
            for (const address of Object.keys(poolContent.queued)) {
                for (const nonce of Object.keys(poolContent.queued[address])) {
                    allTransactions.push(poolContent.queued[address][nonce]);
                }
            }
        }

        console.log(`Total transactions in mempool (pending + queued): ${allTransactions.length}`);

        if (allTransactions.length > 0) {
            console.log("\n--- First 5 Transactions ---");
            const firstFive = allTransactions.slice(0, 5);
            firstFive.forEach((tx, i) => {
                // Formatting nonce safely whether it's hex or dec
                const nonce = tx.nonce.startsWith('0x') ? parseInt(tx.nonce, 16) : tx.nonce;
                console.log(`${i + 1}. Hash: ${tx.hash} | From: ${tx.from} | Nonce: ${nonce}`);
            });

            console.log("\n--- Last 5 Transactions ---");
            // Pick up to 5 elements from the end of the array
            const lastFive = allTransactions.slice(-5);
            lastFive.forEach((tx, i) => {
                const index = allTransactions.length - lastFive.length + i + 1;
                const nonce = tx.nonce.startsWith('0x') ? parseInt(tx.nonce, 16) : tx.nonce;
                console.log(`${index}. Hash: ${tx.hash} | From: ${tx.from} | Nonce: ${nonce}`);
            });
        }
    } catch (error) {
        console.error("Error fetching mempool content:", error);
    }
}

main();
