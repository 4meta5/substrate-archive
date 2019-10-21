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

use log::*;
use futures::{Future, Stream, sync::mpsc::UnboundedSender, future::{self, join_all}};
use tokio::runtime::Runtime;
use jsonrpc_core_client::{RpcChannel, transports::ws};
use substrate_primitives::storage::StorageKey;
use substrate_rpc_primitives::number::NumberOrHex;
use substrate_subxt::{system::System, balances::Balances, Client};

use std::sync::Arc;

use crate::types::{Data, SubstrateBlock, storage::StorageKeyType, Block, Header, Storage};
use crate::error::{Error as ArchiveError};

/// Communicate with Substrate node via RPC
pub struct Rpc<T: System + Balances + 'static> {
    client: Client<T>,
    url: url::Url
}

impl<T> Rpc<T> where T: System + Balances + 'static {

    pub(crate) fn new(rt: Runtime, url: url::Url) -> Self {
        let client = rt.block_on(Client::new()
            .set_url(url)
            .build()).expect("client instantiation should not fail; qed");
        Self { client, url }
    }

    pub(crate) fn metadata(&self) -> () {
        let metadata = self.client.metadata();
        println!("{:?}", metadata);
        future::ok(())
    }

    /// send all new headers back to main thread
    pub(crate) fn subscribe_new_heads(&self, sender: UnboundedSender<Data<T>>
    ) -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client
            .subscribe_blocks()
            .and_then(|stream| {
                stream
                    .for_each(move |head| {
                        sender.unbounded_send(Data::Header(Header::new(head)))
                              .map_err(|e| ArchiveError::from(e))
                    })
            })
    }

    /// send all finalized headers back to main thread
    pub(crate) fn subscribe_finalized_blocks(&self, sender: UnboundedSender<Data<T>>
    ) -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client
            .subscribe_finalized_blocks()
            .and_then(|stream| {
                stream.for_each(move |head| {
                    sender.unbounded_send(Data::FinalizedHead(Header::new(head)))
                          .map_err(Into::into)
                })
            })
    }

    // TODO: make "Key" and "from" vectors
    // TODO: Merge 'from' and 'key' via a macro_derive on StorageKeyType, to auto-generate storage keys
    /// Get a storage item
    /// must provide the key, hash of the block to get storage from, as well as the key type
    pub(crate) fn storage(&self,
                          sender: UnboundedSender<Data<T>>,
                          key: StorageKey,
                          hash: T::Hash,
                          from: StorageKeyType
    ) -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client
            .storage(key, hash)
            .and_then(move |data| {
                if let Some(d) = data {
                    sender
                        .unbounded_send(Data::Storage(Storage::new(d, from, hash)))
                        .map_err(Into::into)
                } else {
                    warn!("Storage Item does not exist!");
                    Ok(())
                }
            })
    }

    /// Fetch a block by hash from Substrate RPC
    pub(crate) fn block(&self, hash: T::Hash, sender: UnboundedSender<Data<T>>
    ) -> impl Future<Item = (), Error = ArchiveError>
    {
        self.client
            .block(hash)
            .and_then(move |block| {
                Self::send_block(block, sender)
            })
    }

    pub(crate) fn block_from_number(&self,
                       number: NumberOrHex<T::BlockNumber>,
                       sender: UnboundedSender<Data<T>>
    ) -> impl Future<Item = (), Error = ArchiveError>
    {
        let client2 = self.client.clone();
        self.client
            .block_hash(number)
            .and_then(move |hash| {
                client2.block(hash.expect("Should always exist"))
                      .and_then(move |block| {
                          Self::send_block(block, sender)
                      })
            })
    }

    pub(crate) fn batch_block_from_number(&self,
                                          numbers: Vec<NumberOrHex<T::BlockNumber>>,
                                          handle: tokio::runtime::TaskExecutor,
                                          sender: UnboundedSender<Data<T>>
    ) -> impl Future<Item = (), Error = ArchiveError>
    {
        let client = Arc::new(self.client.clone());

        let mut futures = Vec::new();
        for number in numbers {
            let client = client.clone();
            let sender = sender.clone();
            futures.push(
                client.block_hash(number)
                      .and_then(move |hash| {
                          client.block(hash.expect("should always exist"))
                                .and_then(move |block| {
                                    Self::send_block(block, sender.clone())
                                })
                      })
            );
        }
        handle.spawn(
            join_all(futures)
                .map_err(|e| error!("{:?}", e))
                .map(|_| ())
        );
        future::ok(())
    }

    fn send_block(block: Option<SubstrateBlock<T>>, sender: UnboundedSender<Data<T>>
    ) -> Result<(), ArchiveError>
    {
        if let Some(b) = block {
            sender.unbounded_send(Data::Block(Block::new(b)))
                .map_err(Into::into)
        } else {
            warn!("No Block Exists!");
            Ok(())
        }
    }

    /// unsubscribe from finalized heads
    fn unsubscribe_finalized_heads() {
        unimplemented!();
    }

    /// unsubscribe from new heads
    fn unsubscribe_new_heads() {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn can_query_blocks() {

    }
}
