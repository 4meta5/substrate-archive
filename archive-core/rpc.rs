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
use futures::{Future, Stream, sync::mpsc, future};
use tokio::runtime::Runtime;
use tokio::util::StreamExt;
use failure::{Error as FailError};
use jsonrpc_core_client::{RpcChannel, transports::ws};
use runtime_primitives::{
    OpaqueExtrinsic as UncheckedExtrinsic,
    generic::{Block as BlockT, SignedBlock}
};
use serde::de::DeserializeOwned;
use substrate_rpc_api::{
    author::AuthorClient,
    chain::{
        ChainClient,
    },
    state::StateClient,
};

use crate::types::{Data, Payload, System, BlockNumber, Block};
use crate::error::{Error as ArchiveError};



pub fn run<T: System + std::fmt::Debug + 'static>() -> Result<(), ArchiveError> {
    // let  (mut rt, client) = client::<T>();
    let (sender, receiver) = mpsc::unbounded();
    let mut rt = Runtime::new()?;
    let rpc = Rpc::<T>::new(&mut rt, &url::Url::parse("ws://127.0.0.1:9944")?)?;
    rt.spawn(rpc.subscribe_new_heads(sender.clone()).map_err(|e| println!("{:?}", e)));
    rt.spawn(rpc.subscribe_finalized_blocks(sender.clone()).map_err(|e| println!("{:?}", e)));
    // rt.spawn(rpc.subscribe_events(sender.clone()).map_err(|e| println!("{:?}", e)));
    tokio::run(receiver.enumerate().for_each(|(i, data)| {
        println!("item: {}, {:?}", i, data);
        future::ok(())
    }));
    Ok(())
}



impl<T: System> From<RpcChannel> for Rpc<T> {
    fn from(channel: RpcChannel) -> Self {
        Self {
            state: channel.clone().into(),
            chain: channel.clone().into(),
            author: channel.into(),
        }
    }
}

/// Communicate with Substrate node via RPC
pub struct Rpc<T: System> {
    state: StateClient<T::Hash>,
    chain: ChainClient<T::BlockNumber, T::Hash, <T as System>::Header, Block<T>>,
    author: AuthorClient<T::Hash, T::Hash>,
}

impl<T> Rpc<T> where T: System + 'static {

    /// instantiate new client
    pub fn new(rt: &mut Runtime, url: &url::Url) -> Result<Self, ArchiveError> {
        rt.block_on(ws::connect(url).map_err(Into::into))
    }

    /// send all new headers back to main thread
    pub fn subscribe_new_heads(&self, sender: mpsc::UnboundedSender<Data<T>>)
                               -> impl Future<Item = (), Error = ArchiveError>
    {
        self.chain.subscribe_new_heads()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |head| {
                    sender.unbounded_send(Data {
                        payload: Payload::Header(head)
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }

    /// send all finalized headers back to main thread
    pub fn subscribe_finalized_blocks(&self, sender: mpsc::UnboundedSender<Data<T>>)
                                      -> impl Future<Item = (), Error = ArchiveError>
    {
        self.chain.subscribe_finalized_heads()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |head| {
                    sender.unbounded_send(Data {
                        payload: Payload::FinalizedHead(head)
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }
/*
    /// send all substrate events back to main thread
    pub fn subscribe_storage(&self, sender: mpsc::UnboundedSender<Data<T>>)
                             -> impl Future<Item = (), Error = ArchiveError>
    {
        self.state.subscribe_storage()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |storage_change| {
                    sender.unbounded_send(Data {
                        payload: Payload::Event(storage_change),
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }
*/
    fn block(&self, hash: Option<BlockNumber<T>>, sender: mpsc::UnboundedSender<Data<T>>)
             -> impl Future<Item = (), Error = ArchiveError> + '_
    {
        self.chain
            .block_hash(hash)
            .map_err(Into::into)
            .and_then(move |h| {
                self.chain
                    .block(h)
                    .map_err(Into::into)
                    .and_then(move |blk| {
                        if let Some(b) = blk {
                            sender.unbounded_send(Data {
                                payload: Payload::Block(b)
                            }).map_err(|e| ArchiveError::from(e))
                        } else {
                            info!("No block with hash {:?}", h);
                            Ok(())
                        }
                    })
            })

/*        self.chain.block()
                   .map_err(|e| ArchiveError::from(e))
                   .and_then(|blk| {

                   })
        */
    }

    fn unsubscribe_finalized_heads() {
        unimplemented!();
    }

    fn unsubscribe_new_heads() {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn can_query_blocks() {

    }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
