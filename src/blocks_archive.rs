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

use crate::{types::System, rpc::Rpc, error::Error, database::Database};

pub struct BlocksArchive<T: System + Debug> {
    db: Arc<Database>,
    rpc: Arc<Rpc<T>>,
    latest: u64,
}

impl<T> BlocksArchive<T> where T: System + Debug {

    /// Create new Blocks Archive
    pub fn new(db: Arc<Database>, rpc: Arc<Rpc<T>>) -> Self {
        Self { db, rpc }
    }

    /// Gather all historical block and commit them to the DB
    pub async fn sync(&mut self) -> Result<(), Error> {
        self.refresh_latest_block();
        let (db, rpc) = (self.db.clone(), self.rpc.clone());

        let blocks = db.query_missing_blocks(Some(latest)).await?;

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
        let len = blocks.len();
        let b = db
            .insert(Data::BatchBlock(BatchBlock::<T>::new(blocks)))
            .await?;
        Ok(())
    }

    async fn verify(&self, latest: Option<u64>) -> Result<bool, Error> {
        let blocks = self.db.query_missing_blocks(latest).await?;
        Ok(blocks.len() == 0)
        // TODO can run other verification tasks, like
        // checking if all extrinsics are in db, too
        // and then print out what is missing
    }

    async fn refresh_latest_block(&mut self) {
        let latest = rpc.clone().header(None).await?;
        log::debug!("Latest Block: {:?}", latest);
        if let Some(h) = latest {
            self.latest = u64::from(h.number())
        }
    }
}
