//! reth-db (MDBX) backed persistent storage for execution witnesses.
//!
//! This is the production database implementation using reth's MDBX-based storage.
//! Witnesses are stored by block number with metadata tracking for chain tip recovery.

use std::{ops::RangeInclusive, path::PathBuf};

use alloy_eips::BlockNumHash;
use alloy_primitives::BlockNumber;
use alloy_rpc_types_debug::ExecutionWitness;
use reth_db::{
    DatabaseEnv, mdbx::{DatabaseArguments, init_db_for},
};
use reth_db_api::{
    cursor::{DbCursorRO, DbCursorRW},
    database::Database as RethDatabase,
    transaction::{DbTx, DbTxMut},
};

use super::{
    Database, DatabaseError,
    codec::{MetaKey, MetaValue, WitnessValue},
    tables::{ExecutionWitnesses, WitnessIndexerTablesWithVersionHistory, WitnessMetadata},
};

/// reth-db (MDBX) backed witness storage.
#[derive(Debug)]
pub struct Db {
    env: DatabaseEnv,
}

impl Db {
    /// Creates new tables required by the witness indexer under the db path.
    ///
    /// Returns a wrapper around [`DatabaseEnv`].
    pub fn new_with_opts(
        path: impl Into<PathBuf>,
        args: DatabaseArguments,
    ) -> Result<Self, DatabaseError> {
        let env = init_db_for::<_, WitnessIndexerTablesWithVersionHistory>(path.into(), args)
            .map_err(|e| DatabaseError::Init(e.to_string()))?;

        Ok(Self { env })
    }
}

#[async_trait::async_trait]
impl Database for Db {
    async fn update_latest_block(&self, latest_block: BlockNumHash) -> Result<(), DatabaseError> {
        let tx = self.env.tx_mut()?;
        tx.put::<WitnessMetadata>(
            MetaKey::LatestBlock.into(),
            MetaValue::latest_block(latest_block),
        )?;
        tx.commit()?;
        Ok(())
    }

    async fn get_latest_block(&self) -> Result<Option<BlockNumHash>, DatabaseError> {
        let tx = self.env.tx()?;
        let result = tx.get::<WitnessMetadata>(MetaKey::LatestBlock.into())?;
        Ok(result.and_then(|v| v.as_latest_block()))
    }

    async fn update_oldest_block(&self, oldest_block: BlockNumber) -> Result<(), DatabaseError> {
        let tx = self.env.tx_mut()?;
        tx.put::<WitnessMetadata>(
            MetaKey::OldestBlock.into(),
            MetaValue::oldest_block(oldest_block),
        )?;
        tx.commit()?;
        Ok(())
    }

    async fn get_oldest_block(&self) -> Result<Option<BlockNumber>, DatabaseError> {
        let tx = self.env.tx()?;
        let result = tx.get::<WitnessMetadata>(MetaKey::OldestBlock.into())?;
        Ok(result.and_then(|v| v.as_oldest_block()))
    }

    async fn insert(
        &self,
        num_hash: BlockNumHash,
        witness: ExecutionWitness,
    ) -> Result<(), DatabaseError> {
        let tx = self.env.tx_mut()?;
        tx.put::<ExecutionWitnesses>(num_hash.number, WitnessValue(witness))?;
        tx.commit()?;
        Ok(())
    }

    async fn get_by_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<ExecutionWitness>, DatabaseError> {
        let tx = self.env.tx()?;
        let result = tx.get::<ExecutionWitnesses>(number)?;
        Ok(result.map(|v| v.0))
    }

    async fn delete(&self, number: BlockNumber) -> Result<(), DatabaseError> {
        let tx = self.env.tx_mut()?;
        tx.delete::<ExecutionWitnesses>(number, None)?;
        tx.commit()?;
        Ok(())
    }

    async fn delete_range(&self, range: RangeInclusive<BlockNumber>) -> Result<(), DatabaseError> {
        let tx = self.env.tx_mut()?;

        // Use cursor for efficient range deletion
        let mut cursor = tx.cursor_write::<ExecutionWitnesses>()?;

        for number in range {
            // Seek to the key and delete if found
            if cursor.seek_exact(number)?.is_some() {
                cursor.delete_current()?;
            }
        }

        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::test_utils::{create_num_hash, create_test_witness};

    use super::*;

    /// Creates a temporary database for testing.
    fn create_test_db() -> eyre::Result<(Db, tempfile::TempDir)> {
        let tempdir = tempfile::tempdir()?;
        let db_path = tempdir.path();
        let db_args = DatabaseArguments::default();
        let db = Db::new_with_opts(db_path, db_args)?;
        Ok((db, tempdir))
    }

    #[tokio::test]
    async fn test_insert_and_get_by_number() -> eyre::Result<()> {
        let (db, _tempdir) = create_test_db()?;
        let num_hash = create_num_hash(42);
        let witness = create_test_witness()?;

        db.insert(num_hash, witness.clone()).await?;

        let retrieved = db.get_by_number(num_hash.number).await?;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), witness);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_nonexistent_by_number() -> eyre::Result<()> {
        let (db, _tempdir) = create_test_db()?;

        let result = db.get_by_number(999).await?;
        assert!(result.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_update_and_get_latest_block() -> eyre::Result<()> {
        let (db, _tempdir) = create_test_db()?;

        // Initially default
        let initial = db.get_latest_block().await?;
        assert!(initial.is_none());

        // Update and verify
        let num_hash = create_num_hash(100);
        db.update_latest_block(num_hash).await?;

        let latest = db.get_latest_block().await?;
        assert_eq!(latest.unwrap().number, num_hash.number);
        assert_eq!(latest.unwrap().hash, num_hash.hash);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_and_get_oldest_block() -> eyre::Result<()> {
        let (db, _tempdir) = create_test_db()?;

        // Initially default
        let initial = db.get_oldest_block().await?;
        assert!(initial.is_none());

        // Update and verify
        db.update_oldest_block(50).await?;

        let oldest = db.get_oldest_block().await?;
        assert_eq!(oldest.unwrap(), 50);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_single() -> eyre::Result<()> {
        let (db, _tempdir) = create_test_db()?;
        let num_hash = create_num_hash(10);
        let witness = create_test_witness()?;

        db.insert(num_hash, witness).await?;

        // Verify it exists
        assert!(db.get_by_number(num_hash.number).await?.is_some());

        // Delete
        db.delete(num_hash.number).await?;

        // Verify removal
        assert!(db.get_by_number(num_hash.number).await?.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_range() -> eyre::Result<()> {
        let (db, _tempdir) = create_test_db()?;

        // Insert blocks 1-5
        for i in 1..=5 {
            let num_hash = create_num_hash(i);
            db.insert(num_hash, create_test_witness()?).await?;
        }

        // Delete range 2-4
        db.delete_range(2..=4).await?;

        // Verify 1 and 5 still exist
        assert!(db.get_by_number(1).await?.is_some());
        assert!(db.get_by_number(5).await?.is_some());

        // Verify 2, 3, 4 are deleted
        assert!(db.get_by_number(2).await?.is_none());
        assert!(db.get_by_number(3).await?.is_none());
        assert!(db.get_by_number(4).await?.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_persistence() -> eyre::Result<()> {
        let tempdir = tempfile::tempdir()?;
        let num_hash = create_num_hash(42);
        let witness = create_test_witness()?;

        // Insert data and close
        {
            let db_path = tempdir.path();
            let db_args = DatabaseArguments::default();
            let db = Db::new_with_opts(db_path, db_args)?;
            db.insert(num_hash, witness.clone()).await?;
            db.update_latest_block(num_hash).await?;
            db.update_oldest_block(10).await?;
        }

        // Reopen and verify data persisted
        {
            let db_path = tempdir.path();
            let db_args = DatabaseArguments::default();
            let db = Db::new_with_opts(db_path, db_args)?;

            let retrieved = db.get_by_number(num_hash.number).await?;
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap(), witness);

            let latest = db.get_latest_block().await?;
            assert_eq!(latest.unwrap().number, num_hash.number);
            assert_eq!(latest.unwrap().hash, num_hash.hash);

            let oldest = db.get_oldest_block().await?;
            assert_eq!(oldest.unwrap(), 10);
        }

        Ok(())
    }
}
