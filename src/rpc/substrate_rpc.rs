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

//! A simple shim over the Substrate Rpc
use codec::Decode;
use futures::{future, Future, Stream};
use jsonrpc_core_client::{transports::ws, RpcChannel};
use log::trace;
use runtime_metadata::RuntimeMetadataPrefixed;
use substrate_primitives::storage::{StorageData, StorageKey};
use substrate_rpc_api::{author::AuthorClient, chain::ChainClient, state::StateClient};
use substrate_rpc_primitives::number::NumberOrHex;

use std::convert::TryInto;

use crate::{
    error::Error as ArchiveError,
    metadata::Metadata,
    types::{SubstrateBlock, System},
};

impl<T: System> From<RpcChannel> for SubstrateRpc<T> {
    fn from(channel: RpcChannel) -> Self {
        Self {
            state: channel.clone().into(),
            chain: channel.clone().into(),
            author: channel.into(),
        }
    }
}

/// Communicate with Substrate node via RPC
pub struct SubstrateRpc<T: System> {
    state: StateClient<T::Hash>,
    chain: ChainClient<T::BlockNumber, T::Hash, <T as System>::Header, SubstrateBlock<T>>,
    #[allow(dead_code)] // TODO remove
    author: AuthorClient<T::Hash, T::Hash>, // TODO get types right
}

impl<T> SubstrateRpc<T>
where
    T: System,
{
    /// instantiate new client
    pub fn connect(url: &url::Url) -> impl Future<Item = Self, Error = ArchiveError> {
        ws::connect(url).map_err(Into::into)
    }

    /// send all new headers back to main thread
    pub(crate) fn subscribe_new_heads(
        &self,
    ) -> impl Future<Item = impl Stream<Item = T::Header, Error = ArchiveError>, Error = ArchiveError>
    {
        self.chain
            .subscribe_new_heads()
            .map(|s| s.map_err(Into::into))
            .map_err(|e| ArchiveError::from(e))
    }

    /// send all finalized headers back to main thread
    pub(crate) fn subscribe_finalized_heads(
        &self,
    ) -> impl Future<Item = impl Stream<Item = T::Header, Error = ArchiveError>, Error = ArchiveError>
    {
        self.chain
            .subscribe_finalized_heads()
            .map(|s| s.map_err(Into::into))
            .map_err(|e| ArchiveError::from(e))
    }

    pub(crate) fn metadata(
        &self,
        hash: Option<T::Hash>,
    ) -> impl Future<Item = Metadata, Error = ArchiveError> {
        self.state
            .metadata(hash)
            .map(|bytes| Decode::decode(&mut &bytes[..]).expect("Decode failed"))
            .map_err(Into::into)
            .and_then(|meta: RuntimeMetadataPrefixed| {
                trace!("Runtime Metadata Prefixed: {:?}", meta);
                future::result(meta.try_into().map_err(Into::into))
            })
    }

    // TODO: make "Key" and "from" vectors
    /// Get a storage item
    /// must provide the key, hash of the block to get storage from, as well as the key type
    pub(crate) fn storage(
        &self,
        key: StorageKey,
        hash: T::Hash,
    ) -> impl Future<Item = Option<StorageData>, Error = ArchiveError> {
        // let hash: Vec<u8> = hash.encode();
        // let hash: T::Hash = Decode::decode(&mut hash.as_slice()).unwrap();
        self.state.storage(key, Some(hash)).map_err(Into::into)
    }

    pub(crate) fn storage_keys(
        &self,
        prefix: StorageKey,
        hash: Option<T::Hash>,
    ) -> impl Future<Item = Vec<StorageKey>, Error = ArchiveError> {
        self.state.storage_keys(prefix, hash).map_err(Into::into)
    }

    /// Fetch a block by hash from Substrate RPC
    pub(crate) fn block(
        &self,
        hash: Option<T::Hash>,
    ) -> impl Future<Item = Option<SubstrateBlock<T>>, Error = ArchiveError> {
        self.chain.block(hash).map_err(Into::into)
    }

    pub(crate) fn hash(
        &self,
        number: NumberOrHex<T::BlockNumber>,
    ) -> impl Future<Item = Option<T::Hash>, Error = ArchiveError> {
        self.chain.block_hash(Some(number)).map_err(Into::into)
    }

    /// unsubscribe from finalized heads
    #[allow(dead_code)]
    fn unsubscribe_finalized_heads() {
        unimplemented!();
    }

    /// unsubscribe from new heads
    #[allow(dead_code)]
    fn unsubscribe_new_heads() {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tests::Runtime, types::*};
    use std::str::FromStr;
    use substrate_primitives::{storage::StorageKey, twox_128};
    use substrate_primitives::{H256, U256};
    use tokio::runtime::Runtime as TokioRuntime;

    fn connect() -> (TokioRuntime, SubstrateRpc<Runtime>) {
        let mut runtime = TokioRuntime::new().unwrap();
        let rpc = runtime
            .block_on(SubstrateRpc::<Runtime>::connect(
                &url::Url::parse("ws://127.0.0.1:9944").unwrap(),
            ))
            .unwrap();
        (runtime, rpc)
    }

    // [WARNING] Needs an Rpc running on port 9944
    #[test]
    fn should_get_block() {
        let (mut rt, rpc) = connect();
        let block = rt
            .block_on(
                rpc.block(
                    "373c569f3520c7ba67a7ac1d6b8e4ead5bd27b1ec28f3e39f5f863c503956e31"
                        .parse()
                        .unwrap(),
                ),
            )
            .unwrap();
        println!("{:?}", block);
    }

    // [WARNING] Requires an Rpc running on port 9944
    #[test]
    fn should_get_hash() {
        let (mut rt, rpc) = connect();
        let hash = rt.block_on(rpc.hash(NumberOrHex::Number(6))).unwrap();
        println!("{:?}", hash);
    }

    #[test]
    fn should_get_storage() {
        let (mut rt, rpc) = connect();
        let timestamp_key = b"Timestamp Now";
        let storage_key = StorageKey(twox_128(timestamp_key).to_vec());
        let hash: <Runtime as System>::Hash =
            "373c569f3520c7ba67a7ac1d6b8e4ead5bd27b1ec28f3e39f5f863c503956e31"
                .parse()
                .unwrap();
        let storage = rt.block_on(rpc.storage(storage_key, hash));
    }
}
