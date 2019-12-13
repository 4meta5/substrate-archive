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

//! Spawning of all tasks happens in this module
//! Nowhere else is anything ever spawned

mod blocks_archive;
mod storage_archive;

use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    future, StreamExt, TryFutureExt,
};
use log::*;
use tokio::runtime::{Builder, Runtime};

use std::sync::Arc;

use self::{blocks_archive::BlocksArchive, storage_archive::StorageArchive};
use crate::{
    database::Database,
    error::Error as ArchiveError,
    rpc::Rpc,
    types::{Data, System},
};

// with the hopeful and long-anticipated release of async-await
pub struct Archive<T: System> {
    rpc: Arc<Rpc<T>>,
    db: Arc<Database>,
    runtime: Runtime,
}

impl<T> Archive<T>
where
    T: System,
{
    pub fn new() -> Result<Self, ArchiveError> {
        let mut runtime = Builder::new()
            .thread_name("archive-worker")
            .num_threads(12)
            .threaded_scheduler()
            .build()?;
        let rpc = runtime.block_on(Rpc::<T>::new(url::Url::parse("ws://127.0.0.1:9944")?))?;
        let (rpc, db) = (Arc::new(rpc), Arc::new(Database::new()?));

        log::debug!("METADATA:\n {}", rpc.metadata());
        log::debug!("KEYS: {:?}", rpc.keys());
        // log::debug!("PROPERTIES: {:?}", rpc.properties());
        Ok(Self { rpc, db, runtime })
    }

    pub fn run(mut self) -> Result<(), ArchiveError> {
        let (sender, receiver) = mpsc::unbounded();
        let data_in = Self::handle_data(receiver, self.db.clone());
        let blocks = Self::blocks(self.rpc.clone(), sender.clone());
        let handle = self
            .runtime
            .spawn(Self::sync(self.rpc.clone(), self.db.clone()));
        self.runtime.block_on(future::join(data_in, blocks));
        self.runtime.block_on(handle);
        log::info!("All Done");
        Ok(())
    }

    /// General block subscription thread
    /// adds to database on-the-fly
    async fn blocks(rpc: Arc<Rpc<T>>, sender: UnboundedSender<Data<T>>) {
        match rpc.subscribe_blocks(sender).await {
            Ok(_) => (),
            Err(e) => error!("{:?}", e),
        };
    }

    /// Sync thread
    /// for syncing missing/historical state/blocks/storage
    async fn sync(rpc: Arc<Rpc<T>>, db: Arc<Database>) {
        let (mut blocks, mut storage) = (
            BlocksArchive::new(db.clone(), rpc.clone()),
            StorageArchive::new(db.clone(), rpc.clone()),
        );
        future::join(blocks.run(), storage.run()).await;
    }

    async fn handle_data(mut receiver: UnboundedReceiver<Data<T>>, db: Arc<Database>) {
        while let Some(data) = receiver.next().await {
            match data {
                Data::SyncProgress(missing_blocks) => {
                    println!("{} blocks missing", missing_blocks);
                }
                c => {
                    let db = db.clone();
                    let fut = async move || db.insert(c).map_err(|e| log::error!("{:?}", e)).await;
                    tokio::spawn(fut());
                }
            }
        }
    }
}
