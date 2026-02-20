//! Error types for the witness indexer RPC API.
//!
//! This module defines the error handling infrastructure for RPC operations,
//! including conversion to JSON-RPC [`ErrorObject`] with appropriate error codes.

use alloy_primitives::{BlockHash, BlockNumber};
use jsonrpsee::types::{
    ErrorObject,
    error::{INTERNAL_ERROR_CODE, INVALID_PARAMS_CODE},
};
use reth_evm::execute::ProviderError;

use crate::db::DatabaseError;

/// Custom JSON-RPC error code indicating that the requested witness was not found.
///
/// This uses the application-defined error code range (`-32000` to `-32099`) as specified
/// by the JSON-RPC 2.0 specification.
const WITNESS_NOT_FOUND_CODE: i32 = -32000;

/// Identifies a block by either its number or hash.
///
/// Used in error messages to indicate which block's witness could not be found.
#[derive(Debug)]
pub enum BlockNumberOrHash {
    /// Block identified by its number.
    Number(BlockNumber),
    /// Block identified by its hash.
    Hash(BlockHash),
}

impl std::fmt::Display for BlockNumberOrHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Number(number) => write!(f, "BlockNumber={number}"),
            Self::Hash(hash) => write!(f, "BlockHash={hash}"),
        }
    }
}

/// Errors that can occur when handling witness RPC requests.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error occurred while accessing the database.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// An error occurred while fetching data from the provider.
    #[error(transparent)]
    Provider(#[from] ProviderError),
    /// The requested witness was not found in the index.
    #[error("witness not found for block: {0}")]
    WitnessNotFound(BlockNumberOrHash),
    /// The caller used a block tag (e.g., "latest") instead of a specific block number.
    #[error("BlockNumberOrTag::Tag variants are not supported, use a specific block number")]
    TagNotSupported,
}

/// Converts [`Error`] into a JSON-RPC [`ErrorObject`] with appropriate error codes:
///
/// - [`Error::Database`] → `INTERNAL_ERROR_CODE` (-32603)
/// - [`Error::WitnessNotFound`] → `WITNESS_NOT_FOUND_CODE` (-32000)
/// - [`Error::TagNotSupported`] → `INVALID_PARAMS_CODE` (-32602)
impl From<Error> for ErrorObject<'static> {
    fn from(value: Error) -> Self {
        match value {
            Error::Database(_) | Error::Provider(_) => {
                ErrorObject::owned(INTERNAL_ERROR_CODE, value.to_string(), None::<u8>)
            }
            Error::WitnessNotFound(_) => {
                ErrorObject::owned(WITNESS_NOT_FOUND_CODE, value.to_string(), None::<u8>)
            }
            Error::TagNotSupported => {
                ErrorObject::owned(INVALID_PARAMS_CODE, value.to_string(), None::<u8>)
            }
        }
    }
}
