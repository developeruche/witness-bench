use clap::Parser;
use eth2_network_config::Eth2NetworkConfig;
use genesis::interop_genesis_state;
use rand::rngs::OsRng;
use rand::RngCore;
use serde_json::json;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use eth2_wallet::bip39::{Mnemonic, Language, Seed as Bip39Seed};
use eth2_wallet::{recover_validator_secret_from_mnemonic, KeyType};
use types::{
    ChainSpec, Config, Epoch, ExecutionPayloadHeader, ExecutionPayloadHeaderElectra, Hash256, Keypair, MainnetEthSpec, ExecutionBlockHash, SecretKey,
};
use ssz::Encode; // Import Encode trait for as_ssz_bytes


#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "data")]
    output_dir: PathBuf,

    #[arg(long, default_value = "1")]
    validator_count: usize,

    #[arg(long, default_value = "1234")]
    chain_id: u64,

    #[arg(long)]
    execution_hash: Option<String>,
}

// Hardcoded mnemonic to match lcli usage in run.sh
const MNEMONIC: &str = "test test test test test test test test test test test junk";

// Helper to generate keypairs from mnemonic
fn generate_keypairs(count: usize) -> Vec<Keypair> {
    let mnemonic = Mnemonic::from_phrase(MNEMONIC, Language::English).expect("Should parse mnemonic");
    let seed = Bip39Seed::new(&mnemonic, "");
    let seed_bytes = seed.as_bytes();

    (0..count).map(|i| {
        let (secret, _) = recover_validator_secret_from_mnemonic(seed_bytes, i as u32, KeyType::Voting).expect("Should recover secret");
        let sk = SecretKey::deserialize(secret.as_bytes()).expect("Should deserialize secret key");
        Keypair::from_components(sk.public_key(), sk)
    }).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let spec = ChainSpec::mainnet();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

    // 1. Generate JWT Secret
    // Only generate if not provided (or overwrite? simpler to always write if output dir doesn't exist)
    fs::create_dir_all(&args.output_dir)?;
    let jwt_path = args.output_dir.join("jwt.hex");
    if !jwt_path.exists() {
        let mut jwt_secret = [0u8; 32];
        OsRng.fill_bytes(&mut jwt_secret);
        let jwt_hex = hex::encode(jwt_secret);
        let mut jwt_file = File::create(&jwt_path)?;
        jwt_file.write_all(b"0x")?;
        jwt_file.write_all(jwt_hex.as_bytes())?;
        println!("Generated JWT secret at {:?}", jwt_path);
    }

    // 2. Generate Validator Keys
    println!("Generating {} validator keys from mnemonic...", args.validator_count);
    let keypairs = generate_keypairs(args.validator_count);

    // 3. Create Execution Payload Header (Simulated Genesis)
    // We assume Electra fork is active at genesis, so we use Electra header.
    let mut payload_header = ExecutionPayloadHeaderElectra::<MainnetEthSpec>::default();
    
    if let Some(ref hash_str) = args.execution_hash {
        let hash_str_clean = hash_str.strip_prefix("0x").unwrap_or(&hash_str);
        let hash_bytes = hex::decode(hash_str_clean)?;
        payload_header.block_hash = ExecutionBlockHash(Hash256::from_slice(&hash_bytes));
        println!("Using provided execution hash: {}", hash_str);
    } else {
        payload_header.block_hash = ExecutionBlockHash(Hash256::repeat_byte(0xAA));
        println!("Using dummy execution hash: {:?}", payload_header.block_hash);
    }
    
    let execution_payload_header = ExecutionPayloadHeader::Electra(payload_header);

    // 4. Generate Beacon State (Genesis SSZ)
    // Using a recent timestamp for genesis
    let genesis_time = now + 30; // 30 seconds from now
    println!("Genesis time set to: {}", genesis_time);

    let mut beacon_state = interop_genesis_state::<MainnetEthSpec>(
        &keypairs,
        genesis_time,
        Hash256::repeat_byte(0x42), // Eth1 block hash
        Some(execution_payload_header.clone()),
        &spec,
    )?;

    // 5. Create Config (config.yaml)
    let mut config = Config::from_chain_spec::<MainnetEthSpec>(&spec);
    
    // Mutate spec for testnet properties if possible
    let mut testnet_spec = spec.clone();
    testnet_spec.altair_fork_epoch = Some(Epoch::new(0));
    testnet_spec.bellatrix_fork_epoch = Some(Epoch::new(0));
    testnet_spec.capella_fork_epoch = Some(Epoch::new(0));
    testnet_spec.deneb_fork_epoch = Some(Epoch::new(0));
    testnet_spec.electra_fork_epoch = Some(Epoch::new(0));
    
    // Also update config to reflect these
    config = Config::from_chain_spec::<MainnetEthSpec>(&testnet_spec);

    // Re-generate genesis with updated spec
    beacon_state = interop_genesis_state::<MainnetEthSpec>(
        &keypairs,
        genesis_time,
        Hash256::repeat_byte(0x42),
        Some(execution_payload_header),
        &testnet_spec,
    )?;

    let network_config = Eth2NetworkConfig {
        deposit_contract_deploy_block: 0,
        boot_enr: None,
        genesis_state_source: eth2_network_config::GenesisStateSource::IncludedBytes,
        genesis_state_bytes: Some(beacon_state.as_ssz_bytes().into()),
        config,
        kzg_trusted_setup: kzg::trusted_setup::get_trusted_setup(), // Uses bundled trusted setup
    };

    network_config.write_to_file(args.output_dir.clone(), true)?;
    println!("Generated Consensus Layer config and genesis at {:?}", args.output_dir);

    // 6. Generate Reth Genesis (genesis.json)
    // This must match the CL side (genesis time, etc)
    // Only write genesis.json in Pass 1 (when execution_hash is not provided)
    // to ensure the hash remains consistent with what Reth initialized.
    if args.execution_hash.is_none() {
        let genesis_json = json!({
            "config": {
                "chainId": args.chain_id,
                "homesteadBlock": 0,
                "eip150Block": 0,
                "eip155Block": 0,
                "eip158Block": 0,
                "byzantiumBlock": 0,
                "constantinopleBlock": 0,
                "petersburgBlock": 0,
                "istanbulBlock": 0,
                "muirGlacierBlock": 0,
                "berlinBlock": 0,
                "londonBlock": 0,
                "arrowGlacierBlock": 0,
                "grayGlacierBlock": 0,
                "mergeNetsplitBlock": 0,
                "shanghaiTime": 0,
                "cancunTime": 0,
                "pragueTime": 0,
                "terminalTotalDifficulty": 0,
                "terminalTotalDifficultyPassed": true
            },
            "nonce": "0x0",
            "timestamp": format!("0x{:x}", genesis_time),
            "extraData": "0x",
            "gasLimit": "0xA7A3582000",
            "difficulty": "0x0",
            "mixHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "coinbase": "0x0000000000000000000000000000000000000000",
            "alloc": {
                "0x123463a4B065722E99115D6c222f267d9cABb524": { "balance": "0x1bc16d674ec80000" }, // 2 ETH
                "0x8943545177806ED17B9F23F0a21ee5948eCaa776": { "balance": "0x3635c9adc5dea00000" }, // 1000 ETH
                "0xE25583099BA105D9ec0A67f5Ae86D90e50036425": { "balance": "0x3635c9adc5dea00000" },
                "0x614561D2d143621E126e87831AEF287678B442b8": { "balance": "0x3635c9adc5dea00000" },
                "0xf93Ee4Cf8c6c40b329b0c0626F28333c132CF241": { "balance": "0x3635c9adc5dea00000" },
                "0x802dCbE1B1A97554B4F50DB5119E37E8e7336417": { "balance": "0x3635c9adc5dea00000" },
                "0xAe95d8DA9244C37CaC0a3e16BA966a8e852Bb6D6": { "balance": "0x3635c9adc5dea00000" },
                "0x2c57d1CFC6d5f8E4182a56b4cf75421472eBAEa4": { "balance": "0x3635c9adc5dea00000" },
                "0x741bFE4802cE1C4b5b00F9Df2F5f179A1C89171A": { "balance": "0x3635c9adc5dea00000" },
                "0xc3913d4D8bAb4914328651C2EAE817C8b78E1f4c": { "balance": "0x3635c9adc5dea00000" },
                "0x65D08a056c17Ae13370565B04cF77D2AfA1cB9FA": { "balance": "0x3635c9adc5dea00000" },
                "0x3e95dFbBaF6B348396E6674C7871546dCC568e56": { "balance": "0x3635c9adc5dea00000" },
                "0x5918b2e647464d4743601a865753e64C8059Dc4F": { "balance": "0x3635c9adc5dea00000" },
                "0x589A698b7b7dA0Bec545177D3963A2741105C7C9": { "balance": "0x3635c9adc5dea00000" },
                "0x4d1CB4eB7969f8806E2CaAc0cbbB71f88C8ec413": { "balance": "0x3635c9adc5dea00000" },
                "0xF5504cE2BcC52614F121aff9b93b2001d92715CA": { "balance": "0x3635c9adc5dea00000" },
                "0xF61E98E7D47aB884C244E39E031978E33162ff4b": { "balance": "0x3635c9adc5dea00000" },
                "0xf1424826861ffbbD25405F5145B5E50d0F1bFc90": { "balance": "0x3635c9adc5dea00000" },
                "0xfDCe42116f541fc8f7b0776e2B30832bD5621C85": { "balance": "0x3635c9adc5dea00000" },
                "0xD9211042f35968820A3407ac3d80C725f8F75c14": { "balance": "0x3635c9adc5dea00000" },
                "0xD8F3183DEF51A987222D845be228e0Bbb932C222": { "balance": "0x3635c9adc5dea00000" },
                "0xafF0CA253b97e54440965855cec0A8a2E2399896": { "balance": "0x3635c9adc5dea00000" }
            },
            "number": "0x0",
            "gasUsed": "0x0",
            "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "baseFeePerGas": "0x7"
        });

        let genesis_json_path = args.output_dir.join("genesis.json");
        let mut genesis_file = File::create(&genesis_json_path)?;
        serde_json::to_writer_pretty(&mut genesis_file, &genesis_json)?;
        println!("Generated Execution Layer genesis at {:?}", genesis_json_path);
    } else {
        println!("Skipping genesis.json generation (Pass 2)");
    } 
    
    Ok(())
}
