//! Codec implementations for database value types.
//!
//! Implements [`Compress`] and [`Decompress`] traits for storing execution witnesses
//! and metadata in the reth-db database.

use alloy_eips::BlockNumHash;
use alloy_primitives::{BlockHash, BlockNumber, bytes};
use alloy_rpc_types_debug::ExecutionWitness;
use reth_db_api::{
    DatabaseError,
    table::{Compress, Decompress},
};

/// Metadata keys for the witness indexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MetaKey {
    /// Latest indexed block (stores `BlockNumHash`).
    LatestBlock = 0,
    /// Oldest block still in storage (stores `BlockNumber`).
    OldestBlock = 1,
}

impl From<MetaKey> for u8 {
    fn from(key: MetaKey) -> Self {
        key as Self
    }
}

/// Metadata value that can hold either [`BlockNumHash`] or [`BlockNumber`].
///
/// - `LatestBlock`: Stores 40 bytes (8-byte number + 32-byte hash)
/// - `OldestBlock`: Stores 8 bytes (block number)
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum MetaValue {
    /// Latest block with number and hash.
    LatestBlock(BlockNumHash),
    /// Oldest block number.
    OldestBlock(BlockNumber),
}

impl MetaValue {
    /// Creates a `LatestBlock` variant.
    pub const fn latest_block(num_hash: BlockNumHash) -> Self {
        Self::LatestBlock(num_hash)
    }

    /// Creates an `OldestBlock` variant.
    pub const fn oldest_block(number: BlockNumber) -> Self {
        Self::OldestBlock(number)
    }

    /// Returns the latest block if this is a `LatestBlock` variant.
    pub const fn as_latest_block(&self) -> Option<BlockNumHash> {
        match self {
            Self::LatestBlock(num_hash) => Some(*num_hash),
            _ => None,
        }
    }

    /// Returns the oldest block number if this is an `OldestBlock` variant.
    pub const fn as_oldest_block(&self) -> Option<BlockNumber> {
        match self {
            Self::OldestBlock(number) => Some(*number),
            _ => None,
        }
    }
}

impl Compress for MetaValue {
    type Compressed = Vec<u8>;

    fn compress_to_buf<B: bytes::BufMut + AsMut<[u8]>>(&self, buf: &mut B) {
        match self {
            Self::LatestBlock(num_hash) => {
                // 1 byte discriminant + 8 bytes number + 32 bytes hash = 41 bytes
                buf.put_u8(MetaKey::LatestBlock as u8);
                buf.put_u64(num_hash.number);
                buf.put_slice(num_hash.hash.as_slice());
            }
            Self::OldestBlock(number) => {
                // 1 byte discriminant + 8 bytes number = 9 bytes
                buf.put_u8(MetaKey::OldestBlock as u8);
                buf.put_u64(*number);
            }
        }
    }
}

impl Decompress for MetaValue {
    fn decompress(value: &[u8]) -> Result<Self, DatabaseError> {
        if value.is_empty() {
            return Err(DatabaseError::Decode);
        }

        let discriminant = value[0];
        let data = &value[1..];

        match discriminant {
            0 => {
                // LatestBlock: 8 bytes number + 32 bytes hash
                if data.len() < 40 {
                    return Err(DatabaseError::Decode);
                }
                let number =
                    u64::from_be_bytes(data[0..8].try_into().map_err(|_| DatabaseError::Decode)?);
                let hash = BlockHash::from_slice(&data[8..40]);
                Ok(Self::LatestBlock(BlockNumHash { number, hash }))
            }
            1 => {
                // OldestBlock: 8 bytes number
                if data.len() < 8 {
                    return Err(DatabaseError::Decode);
                }
                let number =
                    u64::from_be_bytes(data[0..8].try_into().map_err(|_| DatabaseError::Decode)?);
                Ok(Self::OldestBlock(number))
            }
            _ => Err(DatabaseError::Decode),
        }
    }
}

/// Wrapper for [`ExecutionWitness`] with reth-db codec support.
///
/// Serializes using postcard for efficient binary encoding.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WitnessValue(pub ExecutionWitness);

impl Compress for WitnessValue {
    type Compressed = Vec<u8>;

    fn compress_to_buf<B: bytes::BufMut + AsMut<[u8]>>(&self, buf: &mut B) {
        let compressed =
            postcard::to_allocvec(&self.0).expect("witness serialization should not fail");
        buf.put_slice(&compressed);
    }
}

impl Decompress for WitnessValue {
    fn decompress(value: &[u8]) -> Result<Self, DatabaseError> {
        postcard::from_bytes(value)
            .map(WitnessValue)
            .map_err(|_| DatabaseError::Decode)
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::B256;

    use super::*;

    #[test]
    fn test_meta_value_latest_block_roundtrip() -> eyre::Result<()> {
        let num_hash = BlockNumHash {
            number: 12345,
            hash: B256::repeat_byte(0xab),
        };
        let value = MetaValue::latest_block(num_hash);

        let compressed = value.clone().compress();
        let decompressed = MetaValue::decompress(&compressed)?;

        assert_eq!(value, decompressed);
        assert_eq!(decompressed.as_latest_block(), Some(num_hash));

        Ok(())
    }

    #[test]
    fn test_meta_value_oldest_block_roundtrip() -> eyre::Result<()> {
        let number = 54321u64;
        let value = MetaValue::oldest_block(number);

        let compressed = value.clone().compress();
        let decompressed = MetaValue::decompress(&compressed)?;

        assert_eq!(value, decompressed);
        assert_eq!(decompressed.as_oldest_block(), Some(number));

        Ok(())
    }
}
