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
use futures::{Future, Stream, sync::mpsc, future, lazy};
use futures_retry::{RetryPolicy, StreamRetryExt, FutureRetry};
use tokio::{runtime::{self, Runtime}, util::StreamExt};
use failure::{Error as FailError};
use substrate_subxt::{Client as SubstrateClient, ClientBuilder, srml::system::System};
use substrate_rpc_primitives::number::NumberOrHex;
use runtime_primitives::traits::Header;
use std::{
    io::{Error as IoError, ErrorKind as IoKind},
    time::Duration,
    sync::Arc,
    clone::Clone,
    marker::PhantomData
};
use crate::{
    types::{Data, Payload, BlockNumber, Block},
    error::{Error as ArchiveError},
};

pub struct Client<T: System + std::fmt::Debug + 'static> {
    runtime: Runtime,
    _marker: PhantomData<T>
}

impl<T> Client<T> where T: System + std::fmt::Debug + 'static {

    fn new() -> Result<Self, ArchiveError> {
        let runtime = Runtime::new()?;

        Ok(Self {
            runtime,
            _marker: PhantomData
        })
    }

    // temporary util function to get a Substrate Client and Runtime
    fn client() -> impl Future<Item = SubstrateClient<T>, Error = ArchiveError> {
        ClientBuilder::<T>::new().build().map_err(Into::into)
        // let client = rt.block_on(client_future)?;
    }

    fn handle_err(e: ArchiveError) -> RetryPolicy<ArchiveError> {
        match e {
            ArchiveError::Io(io) => {
                match io.kind() {
                    IoKind::Interrupted => RetryPolicy::Repeat,
                    _ => RetryPolicy::WaitRetry(Duration::from_millis(100))
                }
            },
            _ => RetryPolicy::WaitRetry(Duration::from_millis(100))
        }
    }

    pub fn run(mut self) -> Result<(), ArchiveError> {
        let (sender, receiver) = mpsc::unbounded();

        let mut sub_heads = Self::client().and_then(|c| Tasks::subscribe_new_heads(c, sender.clone()));
        let mut sub_fin_blocks = Self::client().and_then(|c| Tasks::subscribe_finalized_blocks(c, sender.clone()));
        let mut sub_events = Self::client().and_then(|c| Tasks::subscribe_events(c, sender.clone()));

        self.runtime.spawn(FutureRetry::new(move || sub_heads, Self::handle_err).map_err(|e| println!("{:?}", e)));
        self.runtime.spawn(FutureRetry::new(move || sub_fin_blocks, Self::handle_err).map_err(|e| println!("{:?}", e)));
        self.runtime.spawn(FutureRetry::new(move || sub_events, Self::handle_err).map_err(|e| println!("{:?}", e)));

        tokio::run(self.handle_data(receiver, sender));
        Ok(())
    }

    fn handle_data(self, receiver: mpsc::UnboundedReceiver<Data<T>>, sender: mpsc::UnboundedSender<Data<T>>)
                   -> impl Future<Item = (), Error = ()> + 'static
    {
        println!("Spawning Receiver");
        receiver.enumerate().for_each(move |(i, data)| {
            match &data.payload {
                Payload::FinalizedHead(header) => {
                    println!("Finalized Header");
                    println!("item: {}, {:?}", i, data);
                },
                Payload::Header(header) => {
                    tokio::spawn(
                        Self::client()
                            .and_then(|c| {
                                Tasks::block(c, header.hash(), sender.clone())
                            }).map_err(|e| println!("{:?}", e))
                    );
                    println!("item: {}, {:?}", i, data);
                    println!("Header");
                }
                Payload::BlockNumber(number) => {
                    println!("Block Number");

                    println!("item: {}, {:?}", i, data);
                },
                Payload::Block(block) => {
                    println!("GOT A Block");
                    println!("item: {}, {:?}", i, data);
                },
                Payload::Event(event) => {
                    println!("Event");

                    println!("item: {}, {:?}", i, data);
                },
                Payload::Hash(hash) => {
                    println!("HASH: {:?}", hash);
                }
                _ => {
                    println!("not handled");
                }
            };
            future::ok(())
        })
    }
}

/// Communicate with Substrate node via RPC
#[derive(Clone)]
pub struct Tasks<T: System> {
    _marker: PhantomData<T>
}

impl<T> Tasks<T> where T: System + 'static {

    /// send all new headers back to main thread
    pub fn subscribe_new_heads(client: SubstrateClient<T>, sender: mpsc::UnboundedSender<Data<T>>)
                               -> impl Future<Item = (), Error = ArchiveError>
    {
        client.subscribe_blocks()
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
    pub fn subscribe_finalized_blocks(client: SubstrateClient<T>, sender: mpsc::UnboundedSender<Data<T>>)
                                      -> impl Future<Item = (), Error = ArchiveError>
    {
        client.subscribe_finalized_blocks()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |head| {
                    sender.unbounded_send(Data {
                        payload: Payload::FinalizedHead(head)
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }

    /// send all substrate events back to main thread
    pub fn subscribe_events(client: SubstrateClient<T>, sender: mpsc::UnboundedSender<Data<T>>)
                            -> impl Future<Item = (), Error = ArchiveError>
    {
        client.subscribe_events()
            .map_err(|e| ArchiveError::from(e))
            .and_then(|stream| {
                stream.map_err(|e| e.into()).for_each(move |storage_change| {
                    sender.unbounded_send(Data {
                        payload: Payload::Event(storage_change),
                    }).map_err(|e| ArchiveError::from(e))
                })
            })
    }

    fn block_hash(client: SubstrateClient<T>, block_number: Option<BlockNumber<T>>, sender: mpsc::UnboundedSender<Data<T>>)
             -> impl Future<Item = (), Error = ArchiveError>
    {
        client
            .block_hash(block_number)
            .map_err(Into::into)
            .and_then(move |hash| {
                if let Some(h) = hash {
                    sender.unbounded_send(Data {
                        payload: Payload::Hash(h)
                    }).map_err(|e| ArchiveError::from(e))
                } else {
                    info!("No Hash Exists!");
                    Ok(()) // TODO Error out
                }
            })
    }

    fn block(client: SubstrateClient<T>, hash: T::Hash, sender: mpsc::UnboundedSender<Data<T>>)
             -> impl Future<Item = (), Error = ArchiveError>
    {
        client
            .block(Some(hash))
            .map_err(Into::into)
            .and_then(move |block| {
                if let Some(b) = block {
                    sender.unbounded_send(Data {
                        payload: Payload::Block(b)
                    }).map_err(|e| ArchiveError::from(e))
                } else {
                    info!("No block exists! (somehow)");
                    Ok(()) // TODO: error Out
                }
            })
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
