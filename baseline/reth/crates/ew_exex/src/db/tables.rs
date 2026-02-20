//! Table definitions for the witness indexer database.
//!
//! Defines the MDBX tables used to store execution witnesses and metadata
//! using reth's [`tables!`] macro.

use std::fmt;

use reth_db::{TableSet, TableType, TableViewer, table::TableInfo, tables};

use super::codec::{MetaValue, WitnessValue};

tables! {
    /// Stores indexer metadata (latest block, oldest block).
    ///
    /// - Key: `u8` (metadata key discriminant)
    /// - Value: [`MetaValue`] (block number or block num+hash)
    table WitnessMetadata {
        type Key = u8;
        type Value = MetaValue;
    }

    /// Maps block number to execution witness.
    ///
    /// - Key: `u64` (block number)
    /// - Value: [`WitnessValue`] (compressed execution witness)
    table ExecutionWitnesses {
        type Key = u64;
        type Value = WitnessValue;
    }
}

pub(super) struct WitnessIndexerTablesWithVersionHistory;

impl TableSet for WitnessIndexerTablesWithVersionHistory {
    fn tables() -> Box<dyn Iterator<Item = Box<dyn TableInfo>>> {
        let witness_indexer_tables = Tables::ALL
            .iter()
            .map(|t| Box::new(*t) as Box<dyn TableInfo>);

        #[cfg(any(test, feature = "test-utils"))]
        {
            Box::new(
                witness_indexer_tables
                    .chain(std::iter::once(
                        Box::new(reth_db_api::Tables::VersionHistory) as Box<dyn TableInfo>,
                    )),
            )
        }

        #[cfg(all(not(test), not(feature = "test-utils")))]
        Box::new(witness_indexer_tables)
    }
}
