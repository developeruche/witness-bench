//! Execution Extension (`ExEx`) for indexing execution witnesses.
//!
//! This module provides [`WitnessIndexer`], a reth `ExEx` that listens to chain state
//! notifications and extracts execution witnesses for each committed block.
//!
//! ## Behavior
//!
//! 1. On startup, sends a [`FinishedHeight`](reth_exex::ExExEvent::FinishedHeight) event
//!    to inform the node of the indexer's current chain tip
//! 2. Optionally spawns a [`Pruner`] task to remove old witnesses
//! 3. Processes chain notifications:
//!    - **Revert**: Deletes witnesses for reverted blocks
//!    - **Commit**: Extracts and stores witnesses for new blocks

use std::{sync::Arc, time::Duration};

use alloy_consensus::BlockHeader;
use alloy_eips::BlockNumHash;
use alloy_primitives::{B256, BlockNumber};
use alloy_rlp::Encodable;
use alloy_rpc_types_debug::ExecutionWitness;
use futures_util::TryStreamExt;
use reth_evm::execute::Executor;
use reth_execution_types::Chain;
use reth_exex::{ExExContext, ExExEvent};
use reth_exex::{ExExHead};
use reth_node_api::{ConfigureEvm, FullNodeComponents, NodePrimitives, NodeTypes};
use reth_node_core::primitives::RecoveredBlock;
use reth_revm::{State, database::StateProviderDatabase, witness::ExecutionWitnessRecord};
use reth_storage_api::{HeaderProvider, StateProviderFactory};
use reth_tracing::tracing::{debug, info, trace};
use tokio::sync::mpsc;

use crate::{
    db::Database,
    pruner::{Pruner, PrunerConfig},
};

/// Configuration for the [`WitnessIndexer`].
#[derive(Debug, Default, clap::Parser)]
pub struct WitnessIndexerConfig {
    /// An optional block number to start indexing from.
    ///
    /// If some, the latest block from database will be ignored and indexing will start from this
    /// configured start block.
    #[arg(
        long = "reth-witness-indexer.start-block",
        help = "Block number to start indexing from, ignoring the latest block in the database"
    )]
    start_block: Option<u64>,
    /// An optional maximum backfill distance in blocks when a new head is detected.
    ///
    /// If not set, backfills the entire range from the latest indexed block to the new head.
    #[arg(
        long = "reth-witness-indexer.max-backfill-distance",
        help = "Maximum distance in blocks to backfill witnesses when a new head is detected. If not set, backfills the entire range from the latest indexed block to the new head."
    )]
    pub max_backfill_distance: Option<u64>,
    /// Boolean to indicate whether or not to include ancestor headers in the execution witness.
    ///
    /// For more information refer [these docs][ref].
    ///
    /// [ref]: https://docs.rs/alloy-rpc-types-debug/latest/alloy_rpc_types_debug/struct.ExecutionWitness.html#structfield.headers
    #[arg(
        long = "reth-witness-indexer.include-headers",
        default_value_t = false,
        help = "Include ancestor headers in the execution witness"
    )]
    include_headers: bool,
    /// An optional pre-built database instance.
    ///
    /// If provided, this takes precedence over [`Self::db`] configuration.
    #[clap(skip)]
    db: Option<Arc<dyn Database>>,
    /// Configuration for the indexer's pruner.
    #[command(flatten)]
    pruner: PrunerConfig,
}

impl WitnessIndexerConfig {
    /// Set the block number to start indexing from.
    pub const fn with_start_block(mut self, start_block: u64) -> Self {
        self.start_block = Some(start_block);
        self
    }

    /// Set a boolean to indicate whether or not to include ancestor headers in the execution
    /// witnesses.
    pub const fn with_headers(mut self, include: bool) -> Self {
        self.include_headers = include;
        self
    }

    /// Configure the indexer's [`Pruner`] with the [periodic
    /// policy][crate::pruner::PruningPolicy::Periodic].
    pub fn with_pruner_periodic(mut self, interval: Duration, n_recent: u64) -> Self {
        self.pruner = PrunerConfig::periodic(interval, n_recent);
        self
    }

    /// Configure the indexer's [`Pruner`] with the [event-driven
    /// policy][crate::pruner::PruningPolicy::EventDriven].
    pub fn with_pruner_event_driven(mut self, rx: mpsc::Receiver<BlockNumber>) -> Self {
        self.pruner = PrunerConfig::event_driven(rx);
        self
    }

    /// Set a pre-built database instance.
    ///
    /// When provided, this takes precedence over the database configuration,
    /// allowing the database to be shared with other components like RPC extensions.
    pub fn with_db(mut self, db: Arc<dyn Database>) -> Self {
        self.db = Some(db);
        self
    }
}

/// Execution Extension that indexes execution witnesses for each block.
///
/// Listens to chain notifications from the reth node and:
/// - Removes witnesses when blocks are reverted
/// - Stores witnesses for newly committed blocks
///
/// Use the builder methods to configure optional pruning before calling [`run`](Self::run).
#[derive(Debug)]
pub struct WitnessIndexer<Node: FullNodeComponents> {
    ctx: ExExContext<Node>,
    db: Arc<dyn Database>,
    pruner: Option<Pruner>,
    include_headers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExExNotificationsConfig {
    /// Skip notifications from the pipeline.
    ///
    /// When true, only `BlockchainTree` notifications will be delivered.
    pub skip_pipeline_notifications: bool,

    /// Maximum blocks to backfill before a notification.
    ///
    /// If the gap between the ExEx head and the next notification exceeds
    /// this limit, only the most recent `max_backfill_distance` blocks will
    /// be backfilled. Older blocks are skipped.
    ///
    /// Set to None for unlimited backfill (backfill all missing blocks).
    pub max_backfill_distance: Option<u64>,
}

impl<Node: FullNodeComponents> WitnessIndexer<Node> {
    /// Returns a new [`WitnessIndexer`] configured with the provided configuration.
    pub async fn new(
        mut ctx: ExExContext<Node>,
        config: WitnessIndexerConfig,
    ) -> eyre::Result<Self> {
        let db = config
            .db
            .ok_or_else(|| eyre::eyre!("reth-witness-indexer: config.db not setup"))?;

        let pruner = config.pruner.build();
        if pruner.is_none() {
            debug!(
                target: "reth-witness-indexer::exex",
                "reth-witness-indexer configured *without* pruner, disk size will keep growing. Pass the flag `--reth-witness-indexer.pruner` to enabled pruning",
            );
        }

        let start = config.start_block.map(|num| BlockNumHash {
            number: num,
            hash: B256::ZERO,
        });

        // // Configure the ctx
        // ctx.notifications.set_with_config(ExExNotificationsConfig {
        //     skip_pipeline_notifications: true,
        //     max_backfill_distance: config.max_backfill_distance,
        // });
        let latest_block = db.get_latest_block().await?;
        let start_block = start.or(latest_block);
        if let Some(block) = start_block {
            debug!(
                target: "reth-witness-indexer::exex",
                block_number = block.number,
                block_hash = %block.hash,
                "starting from configured block"
            );
            ctx.set_notifications_with_head(ExExHead { block });
        };

        Ok(Self {
            ctx,
            db,
            pruner,
            include_headers: config.include_headers,
        })
    }
}

impl<Node: FullNodeComponents> WitnessIndexer<Node> {
    /// Runs the indexer, processing chain notifications indefinitely.
    ///
    /// This method:
    /// 1. Restores chain tip from database and notifies the node
    /// 2. Spawns the pruner task if configured
    /// 3. Enters the main loop processing commit/revert notifications
    pub async fn run(mut self) -> eyre::Result<()> {
        debug!(
            target: "reth-witness-indexer::exex",
            "starting witness indexer"
        );

        if let Some(pruner) = self.pruner.take() {
            let db = Arc::clone(&self.db);

            self.ctx
                .components
                .task_executor()
                .spawn_critical_with_shutdown_signal(
                    "witness-indexer-pruner",
                    |shutdown| async move {
                        pruner.run_until_shutdown(db, shutdown).await;
                    },
                );
        }

        while let Some(notification) = self.ctx.notifications.try_next().await? {
            // Here we have the `old` chain that was reverted.
            if let Some(reverted_chain) = notification.reverted_chain() {
                self.handle_revert(&reverted_chain).await?;
            }

            // Here we have the `new` chain that has been committed.
            if let Some(committed_chain) = notification.committed_chain() {
                self.handle_commit(&committed_chain).await?;

                self.ctx
                    .events
                    .send(ExExEvent::FinishedHeight(committed_chain.tip().num_hash()))?;
            }
        }

        Ok(())
    }

    /// Handles a chain revert by deleting witnesses for all reverted blocks.
    async fn handle_revert(
        &self,
        chain: &Arc<Chain<<Node::Types as NodeTypes>::Primitives>>,
    ) -> eyre::Result<()> {
        let range = chain.range();
        info!(
            target: "reth-witness-indexer::exex",
            start_block = range.start(),
            end_block = range.end(),
            "handling chain revert"
        );

        self.db.delete_range(range).await?;

        let fork_block = chain
            .headers()
            .next()
            .expect("Chain should have at least one block")
            .parent_num_hash();

        self.db.update_latest_block(fork_block).await?;

        debug!(
            target: "reth-witness-indexer::exex",
            block_number = fork_block.number,
            block_hash = %fork_block.hash,
            "chain tip updated after revert"
        );

        Ok(())
    }

    /// Handles a chain commit by extracting and storing witnesses for all new blocks.
    async fn handle_commit(
        &self,
        chain: &Arc<Chain<<Node::Types as NodeTypes>::Primitives>>,
    ) -> eyre::Result<()> {
        let range = chain.range();
        info!(
            target: "reth-witness-indexer::exex",
            start_block = range.start(),
            end_block = range.end(),
            "handling chain commit"
        );

        for block in chain.blocks_iter() {
            debug!(
                target: "reth-witness-indexer::exex",
                block_number = block.number(),
                block_hash = %block.hash(),
                "extracting witness for block"
            );
            let witness = self.extract_witness(block).await?;
            self.db.insert(block.num_hash(), witness).await?;
        }
        let tip = chain.tip().num_hash();
        self.db.update_latest_block(tip).await?;

        debug!(
            target: "reth-witness-indexer::exex",
            block_number = tip.number,
            block_hash = %tip.hash,
            "chain tip updated after commit"
        );

        Ok(())
    }

    /// Extracts the execution witness for a block by re-executing it.
    ///
    /// This re-executes the block against the parent state to capture all accessed
    /// state, bytecodes, and storage keys needed for stateless validation.
    async fn extract_witness(
        &self,
        block: &RecoveredBlock<<<Node::Types as NodeTypes>::Primitives as NodePrimitives>::Block>,
    ) -> eyre::Result<ExecutionWitness> {
        // Get the state at the block's parent.
        let state_provider = self
            .ctx
            .components
            .provider()
            .state_by_block_hash(block.parent_hash())?;
        let db = StateProviderDatabase::new(&state_provider);

        // Create an executor with the node's config at that state.
        let executor = self.ctx.components.evm_config().executor(db);

        // Execute the current block and record state changes.
        let mut witness_record = ExecutionWitnessRecord::default();
        executor.execute_with_state_closure(block, |state: &State<_>| {
            witness_record.record_executed_state(state);
        })?;
        let ExecutionWitnessRecord {
            hashed_state,
            codes,
            keys,
            lowest_block_number,
        } = witness_record;
        let state = state_provider.witness(Default::default(), hashed_state)?;

        // If EIP-2935 is enabled the historical block hashes are already served from state.
        //
        // If not enabled, we fetch only from the smallest block accessed during block execution.
        //
        // [EIP-2935]: https://eips.ethereum.org/EIPS/eip-2935.
        let headers = if self.include_headers {
            let parent = block.number().saturating_sub(1);
            let smallest = match lowest_block_number {
                Some(smallest) => smallest,
                None => parent,
            };
            let range = smallest..=parent;
            trace!(
                target: "reth-witness-indexer::exex",
                smallest_block = range.start(),
                largest_block = range.end(),
                "fetching ancestral headers for witness"
            );
            self.ctx
                .components
                .provider()
                .headers_range(range)?
                .into_iter()
                .map(|header| {
                    let mut serialized_header = Vec::new();
                    header.encode(&mut serialized_header);
                    serialized_header.into()
                })
                .collect()
        } else {
            Default::default()
        };

        Ok(ExecutionWitness {
            state,
            codes,
            keys,
            headers,
        })
    }
}
