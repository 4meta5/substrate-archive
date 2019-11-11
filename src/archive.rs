// Copyright 2017-2019 Parity Technologies (UK) Ltd.
// This file is part of substrate-archive.

// substrate-archive is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// substrate-archive is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with substrate-archive.  If not, see <http://www.gnu.org/licenses/>.

//! Spawning of tasks happens in this module
use log::*;
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    future, StreamExt,
};
use tokio::prelude::*;
use tokio::runtime::Runtime;
use substrate_primitives::{storage::StorageKey, twox_128, U256};
use substrate_rpc_primitives::number::NumberOrHex;

use std::{marker::PhantomData, sync::Arc};

use crate::{
    database::Database,
    error::Error as ArchiveError,
    rpc::Rpc,
    types::{
        storage::{StorageKeyType, TimestampOp},
        Data, System,
    },
};

// with the hopeful and long-anticipated release of async-await
pub struct Archive<T: System> {
    rpc: Rpc<T>,
    db: Database,
    runtime: Runtime,
}

impl<T> Archive<T>
where
    T: System,
{

    pub fn new() -> Result<Self, ArchiveError> {
        let db = Database::new()?;
        info!("Initializing RPC");
        let mut runtime = Runtime::new()?;
        let rpc = runtime.block_on(Rpc::<T>::new(url::Url::parse("ws://127.0.0.1:9944")?))?;
        // let metadata = runtime.block_on(rpc.metadata())?;
        // debug!("METADATA: {:?}", metadata);
        Ok(Self { rpc, db, runtime })
    }

    pub fn run(mut self) -> Result<(), ArchiveError> {
        let (sender, receiver) = mpsc::unbounded();
        crate::util::init_logger(log::LevelFilter::Error); // TODO user should decide log strategy

        // crawls database and retrieves any missing values
        self.runtime.block_on(Sync::sync(&self.db, &self.rpc, sender.clone()))?;

        let executor = self.runtime.executor();
        let tasks = future::try_join(
            // block subscription thread
            executor.spawn(self.rpc.subscribe_blocks(sender)),
            // inserts into database / handles misc. data (ie sync progress, etc)
            executor.spawn(Self::handle_data(receiver, self.db))
        );

        self.runtime.block_on(async {
            tasks.await.map(|_| ())
        })
    }

    async fn handle_data(mut receiver: UnboundedReceiver<Data<T>>, db: Database
    ) -> Result<(), ArchiveError> {
        for data in receiver.next().await {
            db.insert(data).await?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Sync<T: System + std::fmt::Debug> {
    _marker: PhantomData<T>,
}

impl<T> Sync<T>
where
    T: System + std::fmt::Debug,
{

    async fn sync(
        db: &Database,
        rpc: &Rpc<T>,
        sender: UnboundedSender<Data<T>>,
    ) -> Result<((),()), ArchiveError> {
        future::try_join(
            Self::sync_blocks(db, rpc, sender.clone()),
            Self::sync_timestamps(db, rpc, sender.clone()),
        ).await
    }

    /// find missing timestamps and add them to DB if found. Return number missing timestamps
    async fn sync_timestamps(
        db: &Database,
        rpc: &Rpc<T>,
        sender: UnboundedSender<Data<T>>,
    ) -> Result<(), ArchiveError> {
        let hashes = db.query_missing_timestamps::<T>().await?;
        let timestamp_key = b"Timestamp Now";
        let storage_key = twox_128(timestamp_key);
        let keys = std::iter::repeat(StorageKey(storage_key.to_vec()))
            .take(hashes.len())
            .collect::<Vec<StorageKey>>();
        let key_types = std::iter::repeat(StorageKeyType::Timestamp(TimestampOp::Now))
            .take(hashes.len())
            .collect::<Vec<StorageKeyType>>();
        rpc.batch_storage(sender, keys, hashes, key_types).await?;
        Ok(())
    }

    /// sync blocks, 100,000 at a time, returning number of blocks that were missing before synced
    async fn sync_blocks(
        db: &Database,
        rpc: &Rpc<T>,
        sender: UnboundedSender<Data<T>>,
    ) -> Result<(), ArchiveError> {
        let missing_blocks = db.query_missing_blocks().await?;
        for chunk in missing_blocks.chunks(100_000) {
            let chunk = chunk
                .into_iter()
                .map(|b| NumberOrHex::Hex(U256::from(*b)))
                .collect::<Vec<NumberOrHex<T::BlockNumber>>>();
            rpc.batch_block_from_number(chunk, sender.clone()).await?;
        }
        Ok(())
    }
}
