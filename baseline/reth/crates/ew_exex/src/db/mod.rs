use std::ops::RangeInclusive;

use alloy_eips::BlockNumHash;
use alloy_primitives::BlockNumber;
use alloy_rpc_types_debug::ExecutionWitness;

mod codec;
pub use codec::{MetaKey, MetaValue, WitnessValue};

mod db_reth;
pub use db_reth::Db as RethDb;

mod tables;

/// Errors that can occur during database operations.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// Error during database initialization.
    #[error("error during db initialization: {0}")]
    Init(String),
    /// Error from the underlying reth-db (MDBX) storage.
    #[error(transparent)]
    RethDb(#[from] reth_db_api::DatabaseError),
    /// Error during serialization/deserialization of witness data.
    #[error(transparent)]
    Postcard(#[from] postcard::Error),
}

/// Storage interface for execution witnesses.
///
/// Implementations must be thread-safe (`Sync + Send`) and support async operations.
/// Witnesses are indexed by both block number and block hash for efficient retrieval.
#[async_trait::async_trait]
pub trait Database: Sync + Send + 'static + std::fmt::Debug {
    /// Updates the latest known block in the database.
    async fn update_latest_block(&self, num_hash: BlockNumHash) -> Result<(), DatabaseError>;

    /// Returns the latest known block, or [`BlockNumHash::default()`] if empty.
    async fn get_latest_block(&self) -> Result<Option<BlockNumHash>, DatabaseError>;

    /// Updates the oldest block number still stored in the database.
    async fn update_oldest_block(&self, number: BlockNumber) -> Result<(), DatabaseError>;

    /// Returns the oldest block number stored, or `0` if unknown.
    async fn get_oldest_block(&self) -> Result<Option<BlockNumber>, DatabaseError>;

    /// Inserts an execution witness for the given block.
    async fn insert(
        &self,
        num_hash: BlockNumHash,
        witness: ExecutionWitness,
    ) -> Result<(), DatabaseError>;

    /// Retrieves an execution witness by block number.
    async fn get_by_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<ExecutionWitness>, DatabaseError>;

    /// Deletes the witness for a single block.
    async fn delete(&self, number: BlockNumber) -> Result<(), DatabaseError>;

    /// Deletes witnesses for all blocks in the given range (inclusive).
    async fn delete_range(&self, range: RangeInclusive<BlockNumber>) -> Result<(), DatabaseError>;
}

/// Test utilities for creating mock witnesses and block identifiers.
///
/// These utilities are available for integration tests and external test suites
/// when the `test-utils` feature is enabled.
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use std::sync::{Mutex, OnceLock};

    use alloy_eips::BlockNumHash;
    use alloy_primitives::{BlockHash, Bytes};
    use alloy_rpc_types_debug::ExecutionWitness;
    use eyre::{Result, eyre};
    use rand::{Rng, SeedableRng, rngs::StdRng};

    /// Global RNG singleton, initialized once from SEED env var or random.
    static RNG: OnceLock<Mutex<StdRng>> = OnceLock::new();

    /// Returns a reference to the global RNG, initializing it on first call.
    fn get_rng() -> &'static Mutex<StdRng> {
        RNG.get_or_init(|| {
            let seed: u64 = std::env::var("SEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(rand::random);
            Mutex::new(StdRng::seed_from_u64(seed))
        })
    }

    /// Creates a test witness with randomized values.
    ///
    /// Uses a global RNG seeded from `SEED` environment variable for reproducibility.
    /// If `SEED` is not set, a random seed is used on first call.
    ///
    /// - State: 1-64 entries, each 1..=64 bytes
    /// - Keys: 1-8 entries, each 32 bytes
    /// - Codes: 1-8 entries, each 32..=128 bytes
    /// - Headers: empty vector
    pub fn create_test_witness() -> Result<ExecutionWitness> {
        let mut rng = get_rng()
            .lock()
            .map_err(|e| eyre!("failed to acquire RNG lock: {}", e))?;

        let state: Vec<Bytes> = (0..rng.random_range(1..=64))
            .map(|_| {
                let len = rng.random_range(1..=64);
                let bytes: Vec<u8> = (0..len).map(|_| rng.random()).collect();
                Bytes::from(bytes)
            })
            .collect();

        let keys: Vec<Bytes> = (0..rng.random_range(1..=8))
            .map(|_| {
                let bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
                Bytes::from(bytes)
            })
            .collect();

        let codes: Vec<Bytes> = (0..rng.random_range(1..=8))
            .map(|_| {
                let len = rng.random_range(32..=128);
                let bytes: Vec<u8> = (0..len).map(|_| rng.random()).collect();
                Bytes::from(bytes)
            })
            .collect();

        Ok(ExecutionWitness {
            state,
            codes,
            keys,
            headers: vec![],
        })
    }

    /// Creates a `BlockNumHash` from a block number with a random hash.
    pub fn create_num_hash(number: u64) -> BlockNumHash {
        BlockNumHash {
            number,
            hash: BlockHash::random(),
        }
    }
}
