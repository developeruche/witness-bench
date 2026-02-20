//! The module implements various pruning policies for the data stored by the indexer.

use std::{sync::Arc, time::Duration};

use alloy_primitives::BlockNumber;
use reth_tasks::shutdown::Shutdown;
use reth_tracing::tracing::{debug, error, info};
use tokio::sync::mpsc;

use crate::db::{Database, DatabaseError};

/// The default interval (in seconds) to periodically prune witness database.
const DEFAULT_PRUNING_INTERVAL: u64 = 3600;

/// The default number of recent blocks to maintain witnesses for.
const DEFAULT_PRUNING_N_RECENT: u64 = 256;

/// Configuration for the [`Pruner`].
#[derive(Debug, Default, clap::Args)]
pub struct PrunerConfig {
    /// An optional receiver for the block number.
    ///
    /// Refer [`PruningPolicy::EventDriven`].
    #[clap(skip)]
    rx: Option<mpsc::Receiver<BlockNumber>>,
    /// Enable the pruner with default or custom values.
    #[arg(
        long = "reth-witness-indexer.pruner",
        default_value_t = false,
        help = "Enable the pruner to periodically remove old witnesses"
    )]
    enabled: bool,
    /// The interval duration (in seconds) to periodically prune.
    ///
    /// Refer [`PruningPolicy::Periodic`].
    #[arg(
        long = "reth-witness-indexer.pruner.interval",
        help = format!("Interval in seconds between periodic prune cycles [default: {}]", DEFAULT_PRUNING_INTERVAL)
    )]
    interval: Option<u64>,
    /// The number of recent blocks to preserve witnesses for in case of [`PruningPolicy::Periodic`].
    #[arg(
        long = "reth-witness-indexer.pruner.n-recent",
        help = format!("Number of most recent blocks to keep witnesses for [default: {}]", DEFAULT_PRUNING_N_RECENT)
    )]
    n_recent: Option<u64>,
}

impl PrunerConfig {
    /// Returns a pruner configuration for a periodic pruner.
    pub const fn periodic(interval: Duration, n_recent: u64) -> Self {
        Self {
            rx: None,
            enabled: true,
            interval: Some(interval.as_secs()),
            n_recent: Some(n_recent),
        }
    }

    /// Returns a pruner configuration for a event-driven pruner.
    pub fn event_driven(rx: mpsc::Receiver<BlockNumber>) -> Self {
        Self {
            rx: Some(rx),
            enabled: true,
            ..Default::default()
        }
    }

    /// Returns `true` if the pruner is enabled via CLI flag, explicit config, or if any pruner
    /// argument was provided.
    pub const fn is_enabled(&self) -> bool {
        self.enabled || self.rx.is_some() || self.interval.is_some() || self.n_recent.is_some()
    }

    /// Consumes the config and builds the [Pruner], returning `None` if not enabled.
    pub fn build(self) -> Option<Pruner> {
        if !self.is_enabled() {
            return None;
        }

        Some(match self.rx {
            Some(rx) => Pruner::event_driven(rx),
            None => Pruner::periodic(
                Duration::from_secs(self.interval.unwrap_or(DEFAULT_PRUNING_INTERVAL)),
                self.n_recent.unwrap_or(DEFAULT_PRUNING_N_RECENT),
            ),
        })
    }
}

/// Strategy for pruning old witnesses from the database.
#[derive(Debug)]
pub enum PruningPolicy {
    /// Prunes witnesses up to the block number received on the channel.
    ///
    /// Useful for zk-rollups where pruning is triggered when state is finalized on L1.
    EventDriven {
        /// Channel receiving block numbers to prune up to.
        rx: mpsc::Receiver<BlockNumber>,
    },
    /// Periodically prunes witnesses to retain only the most recent blocks.
    Periodic {
        /// How often to run the pruning check.
        interval: Duration,
        /// Number of recent blocks to keep.
        n_recent: u64,
    },
}

impl std::fmt::Display for PruningPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventDriven { .. } => write!(f, "event driven"),
            Self::Periodic { interval, n_recent } => {
                write!(
                    f,
                    "periodic (interval: {}s, n_recent: {})",
                    interval.as_secs(),
                    n_recent
                )
            }
        }
    }
}

/// Background task that removes old witnesses according to a [`PruningPolicy`].
#[derive(Debug)]
pub struct Pruner {
    policy: PruningPolicy,
}

impl std::fmt::Display for Pruner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pruner with policy: {}", self.policy)
    }
}

impl Pruner {
    /// Returns a pruner with [`PruningPolicy::EventDriven`].
    pub const fn event_driven(rx: mpsc::Receiver<BlockNumber>) -> Self {
        Self {
            policy: PruningPolicy::EventDriven { rx },
        }
    }

    /// Returns a pruner with [`PruningPolicy::Periodic`].
    pub const fn periodic(interval: Duration, n_recent: u64) -> Self {
        Self {
            policy: PruningPolicy::Periodic { interval, n_recent },
        }
    }
}

impl Pruner {
    /// Runs the pruner until the shutdown signal is received.
    ///
    /// Errors during pruning are logged rather than propagated.
    pub async fn run_until_shutdown(mut self, db: Arc<dyn Database>, shutdown: Shutdown) {
        let mut oldest = match db.get_oldest_block().await {
            Ok(oldest) => oldest,
            Err(e) => {
                error!(target: "reth-witness-indexer::pruner", %e, "failed to get oldest block");
                return;
            }
        };

        match self.policy {
            PruningPolicy::EventDriven { ref mut rx } => {
                tokio::pin!(shutdown);
                loop {
                    tokio::select! {
                        biased;
                        _ = &mut shutdown => {
                            debug!(target: "reth-witness-indexer::pruner", "shutdown signal received");
                            break;
                        }
                        block_number = rx.recv() => {
                            match block_number {
                                Some(bn) => {
                                    match Self::prune_up_to_block(&db, oldest, bn).await {
                                        Ok(Some(new_oldest)) => {
                                            oldest = Some(new_oldest);
                                        }
                                        Ok(None) => {
                                            // No pruning was needed, oldest block is already past requested block number
                                        }
                                        Err(e) => {
                                            error!(target: "reth-witness-indexer::pruner", %e, block_number = bn, "prune failed");
                                        }
                                    }
                                }
                                None => {
                                    debug!(target: "reth-witness-indexer::pruner", "channel closed");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            PruningPolicy::Periodic { interval, n_recent } => {
                tokio::pin!(shutdown);
                loop {
                    tokio::select! {
                        biased;
                        _ = &mut shutdown => {
                            debug!(target: "reth-witness-indexer::pruner", "shutdown signal received");
                            break;
                        }
                        _ = tokio::time::sleep(interval) => {
                            match Self::prune_periodic_cycle(&db, oldest, n_recent).await {
                                Ok(Some(new_oldest)) => {
                                    oldest = Some(new_oldest);
                                }
                                Ok(None) => {
                                    // No pruning was needed, oldest block is already past requested block number
                                }
                                Err(e) => {
                                    error!(target: "reth-witness-indexer::pruner", %e, "periodic prune failed");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Processes a single event-driven prune request.
    async fn prune_up_to_block(
        db: &Arc<dyn Database>,
        oldest: Option<BlockNumber>,
        block_number: BlockNumber,
    ) -> Result<Option<u64>, DatabaseError> {
        let oldest = oldest.unwrap_or_default();
        if oldest <= block_number {
            info!(
                target: "reth-witness-indexer::pruner",
                from_block = oldest,
                to_block = block_number,
                "pruning witnesses (event-driven)"
            );
            db.delete_range(oldest..=block_number).await?;
            let new_oldest = block_number.saturating_add(1);
            db.update_oldest_block(new_oldest).await?;
            return Ok(Some(new_oldest));
        }
        debug!(
            target: "reth-witness-indexer::pruner",
            oldest_block = oldest,
            requested_block = block_number,
            "skipping prune, oldest block already past requested"
        );
        Ok(None)
    }

    /// Performs a single periodic pruning cycle.
    async fn prune_periodic_cycle(
        db: &Arc<dyn Database>,
        oldest: Option<BlockNumber>,
        n_recent: u64,
    ) -> Result<Option<u64>, DatabaseError> {
        let oldest = oldest.unwrap_or_default();
        let latest = db.get_latest_block().await?.unwrap_or_default().number;
        let prune_up_to = latest.saturating_sub(n_recent);
        if oldest < prune_up_to {
            info!(
                target: "reth-witness-indexer::pruner",
                from_block = oldest,
                to_block = prune_up_to,
                latest_block = latest,
                n_recent,
                "pruning witnesses (periodic)"
            );
            db.delete_range(oldest..=prune_up_to).await?;
            let new_oldest = prune_up_to.saturating_add(1);
            db.update_oldest_block(new_oldest).await?;
            return Ok(Some(new_oldest));
        }
        debug!(
            target: "reth-witness-indexer::pruner",
            oldest_block = oldest,
            prune_up_to,
            latest_block = latest,
            "skipping periodic prune, nothing to prune"
        );
        Ok(None)
    }
}
