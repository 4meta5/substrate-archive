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

use crate::{types::{System, Data, BatchBlock}, rpc::Rpc, error::Error, database::Database};
use futures::future;
use substrate_rpc_primitives::{number::NumberOrHex};
use runtime_primitives::traits::Header as _;
use substrate_primitives::U256;
use std::sync::Arc;

pub struct BlocksArchive<T: System> {
    db: Arc<Database>,
    rpc: Arc<Rpc<T>>,
    latest: u64,
}

impl<T> BlocksArchive<T> where T: System {

    /// Create new Blocks Archive
    pub fn new(db: Arc<Database>, rpc: Arc<Rpc<T>>) -> Self {
        Self { db, rpc, latest: 0 }
    }

    /// Runs the 'sync', gathering historical blocks
    pub async fn run(&mut self) {
        match self.sync_loop().await {
            Err(e) => log::error!("{:?}", e),
            Ok(_) => ()
        };
    }

    async fn sync_loop(&mut self) -> Result<(), Error> {
        'sync: loop {
            self.sync().await?;
            if self.verify(Some(self.latest)).await? {
                break 'sync;
            } else {
                continue 'sync;
            }
        }
        Ok(())
    }

    /// Gather all historical block and commit them to the DB
    async fn sync(&mut self) -> Result<(), Error> {
        self.refresh_latest_block().await?;
        let (db, rpc) = (self.db.clone(), self.rpc.clone());

        let blocks = db.query_missing_blocks(Some(self.latest)).await?;

        // TODO there is probably a better way of doing this further down in the stack
        let mut futures = Vec::new();
        for chunk in blocks.chunks(100_000) {
            let b = chunk
                .iter()
                .map(|b| NumberOrHex::Hex(U256::from(*b)))
                .collect::<Vec<NumberOrHex<T::BlockNumber>>>();
            futures.push(rpc.batch_block_from_number(b));
        }

        let mut blocks = Vec::new();
        for chunk in future::join_all(futures).await.into_iter() {
            blocks.extend(chunk?.into_iter());
        }

        log::info!("inserting {} blocks", blocks.len());
        let b = db
            .insert(Data::BatchBlock(BatchBlock::<T>::new(blocks)))
            .await?;
        Ok(())
    }

    pub async fn verify(&self, latest: Option<u64>) -> Result<bool, Error> {
        let latest = if let Some(h) = latest {
            h
        } else {
            (*self.rpc.header(None).await?
                .expect("Should always be a latest block; qed")
                .number()).into()
        };
        let blocks = self.db.query_missing_blocks(Some(latest)).await?;
        Ok(blocks.len() == 0)
        // TODO can run other verification tasks, like
        // checking if all extrinsics are in db
        // and then report
    }

    async fn refresh_latest_block(&mut self) -> Result<(), Error> {
        let latest = self.rpc.header(None).await?;
        log::debug!("Latest Block: {:?}", latest);
        if let Some(h) = latest {
            self.latest = (*h.number()).into();
        }
        Ok(())
    }
}
