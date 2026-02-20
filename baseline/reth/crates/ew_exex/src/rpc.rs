//! JSON-RPC API for fetching indexed execution witnesses.
//!
//! This module exposes RPC endpoints under the `indexed` namespace to retrieve
//! [`ExecutionWitness`] by block number or block hash.
//!
//! ## RPC Methods
//!
//! - `indexed_witnessByNumber` - Fetch witness by block number
//! - `indexed_witnessByBlockHash` - Fetch witness by block hash
//!
//! ## Architecture
//!
//! Hash-based lookups use reth's [`reth_storage_api::HeaderProvider`] to convert block hash to block number,
//! then query the witness database by number. This eliminates the need for a separate
//! hash-to-witness index in the database.

use std::sync::Arc;

use alloy_eips::BlockNumberOrTag;
use alloy_primitives::BlockHash;
use alloy_rpc_types_debug::ExecutionWitness;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};
use reth_storage_api::BlockNumReader;
use reth_tracing::tracing::{debug, error};

use crate::{
    db::Database,
    error::{BlockNumberOrHash, Error},
};

/// RPC trait defining the witness indexer API.
///
/// Generates `IndexedWitnessRpcApiServer` for server implementations.
#[rpc(server, namespace = "indexed")]
pub trait IndexedWitnessRpcApi {
    /// Returns the execution witness for the given block number.
    #[method(name = "witnessByNumber")]
    async fn indexed_witness_by_number(
        &self,
        block: BlockNumberOrTag,
    ) -> RpcResult<ExecutionWitness>;

    /// Returns the execution witness for the given block hash.
    #[method(name = "witnessByBlockHash")]
    async fn indexed_witness_by_block_hash(&self, hash: BlockHash) -> RpcResult<ExecutionWitness>;
}

/// RPC service implementation for serving indexed execution witnesses.
///
/// Uses a [`BlockNumReader`] provider for hash-to-number conversion and a [`Database`]
/// for witness storage. This architecture eliminates the need for a hash-to-witness
/// index in the database.
#[derive(Debug)]
pub struct WitnessServiceRpc<P> {
    provider: P,
    db: Arc<dyn Database>,
}

impl<P> WitnessServiceRpc<P> {
    /// Creates a new RPC service with the given provider and database.
    pub const fn new(provider: P, db: Arc<dyn Database>) -> Self {
        Self { provider, db }
    }
}

/// Implementation of [`IndexedWitnessRpcApiServer`] for [`WitnessServiceRpc`].
#[async_trait::async_trait]
impl<P> IndexedWitnessRpcApiServer for WitnessServiceRpc<P>
where
    P: BlockNumReader + Send + Sync + 'static,
{
    async fn indexed_witness_by_number(
        &self,
        block: BlockNumberOrTag,
    ) -> RpcResult<ExecutionWitness> {
        debug!(target: "reth-witness-indexer::rpc", ?block, "indexed_witnessByNumber request");

        let BlockNumberOrTag::Number(number) = block else {
            error!(target: "reth-witness-indexer::rpc", "block tag not supported");
            return Err(Error::TagNotSupported.into());
        };

        self.db
            .get_by_number(number)
            .await
            .map_err(Error::Database)?
            .ok_or(Error::WitnessNotFound(BlockNumberOrHash::Number(number)).into())
    }

    async fn indexed_witness_by_block_hash(&self, hash: BlockHash) -> RpcResult<ExecutionWitness> {
        debug!(target: "reth-witness-indexer::rpc", %hash, "indexed_witnessByBlockHash request");

        // Use the provider to get the block number for the provided block hash.
        let number = self
            .provider
            .block_number(hash)
            .map_err(Error::Provider)?
            .ok_or(Error::WitnessNotFound(BlockNumberOrHash::Hash(hash)))?;

        debug!(target: "reth-witness-indexer::rpc", %hash, block_number = number, "resolved block hash to number");

        // Query by number
        self.db
            .get_by_number(number)
            .await
            .map_err(Error::Database)?
            .ok_or(Error::WitnessNotFound(BlockNumberOrHash::Hash(hash)).into())
    }
}
