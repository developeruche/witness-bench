import { SpamOrchestrator } from '@developeruche/tx-spammer-sdk/dist/SpamOrchestrator';
import { SpamSequenceConfig } from '@developeruche/tx-spammer-sdk/dist/types';
import { createPublicClient, createWalletClient, http, parseEther } from 'viem';
import { SPAMMER_ABI, SPAMMER_BYTECODE } from './SpammerArtifact';
import { privateKeyToAccount } from 'viem/accounts';
import { foundry } from 'viem/chains';

async function main() {
    const RPC_URL = process.env.RPC_URL || 'http://127.0.0.1:55835';
    const ROOT_PRIVATE_KEY =
        process.env.PRIVATE_KEY ||
        '0xbcdf20249abf0ed6d944c0288fad489e33f66b3960d9e6229c1cd214ed3bbe31';

    const chain = { ...foundry, rpcUrls: { default: { http: [RPC_URL] } } };
    const publicClient = createPublicClient({ chain, transport: http(RPC_URL) });
    const rootAccount = privateKeyToAccount(ROOT_PRIVATE_KEY as `0x${string}`);
    const rootClient = createWalletClient({
        account: rootAccount,
        chain,
        transport: http(RPC_URL),
    });

    console.log('\n--- 0. Setup: Deploying Spammer.sol Target ---');
    const deployHash = await rootClient.deployContract({
        abi: SPAMMER_ABI,
        bytecode: SPAMMER_BYTECODE,
    });
    console.log('Deploy tx sent:', deployHash);
    const receipt = await publicClient.waitForTransactionReceipt({ hash: deployHash });
    const spammerAddress = receipt.contractAddress!;
    console.log('Spammer deployed at:', spammerAddress);

    // wait extra 12 seconds just to be sure block prod has progressed
    await new Promise(resolve => setTimeout(resolve, 12000));

    const mixedConfig: SpamSequenceConfig = {
        rpcUrl: RPC_URL,
        chainId: 31337,
        maxGasLimit: 30_000_000n,
        concurrency: 50,
        durationSeconds: 10,
        strategy: {
            mode: 'mixed',
            strategies: [
                {
                    percentage: 2,
                    config: {
                        mode: 'transfer',
                        amountPerTx: parseEther('0.0001'),
                        depth: 1,
                    },
                },
                {
                    percentage: 50,
                    config: {
                        mode: 'write',
                        targetContract: spammerAddress,
                        functionName: 'write_one',
                        abi: SPAMMER_ABI as any,
                        staticArgs: [],
                    },
                },
                {
                    percentage: 48,
                    config: {
                        mode: 'deploy',
                        bytecode: SPAMMER_BYTECODE,
                        args: [],
                    },
                }
            ],
        },
    };

    const orchestrator = new SpamOrchestrator(mixedConfig, ROOT_PRIVATE_KEY as `0x${string}`);

    try {
        await orchestrator.setup(parseEther('1'));
        await orchestrator.start();
    } catch (error) {
        console.error('Spam failed:', error);
    }
}

main();
