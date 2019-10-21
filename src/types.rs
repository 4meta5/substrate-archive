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

pub mod storage;

use substrate_primitives::storage::StorageChangeSet;
use serde::de::DeserializeOwned;
use codec::{ Encode, Decode, EncodeAsRef, HasCompact};
use custom_derive::*;
use enum_derive::*;
use substrate_primitives::storage::StorageData;
use runtime_support::Parameter;
use substrate_subxt::system::System;
use runtime_primitives::{
    OpaqueExtrinsic,
    AnySignature,
    generic::{
        UncheckedExtrinsic,
        Block as BlockT,
        SignedBlock
    },
    traits::{
        Bounded,
        CheckEqual,
        Hash,
        Header as HeaderTrait,
        MaybeDisplay,
        MaybeSerializeDebug,
        MaybeSerializeDebugButNotDeserialize,
        Member,
        SignedExtension,
        SimpleArithmetic,
        SimpleBitOps,
        StaticLookup,
    },
};

use self::storage::StorageKeyType;

#[derive(Clone)]
pub struct Encoded(pub Vec<u8>);

impl Encode for Encoded {
    fn encode(&self) -> Vec<u8> {
        self.0.to_owned()
    }
}

pub fn compact<T: HasCompact>(t: T) -> Encoded {
    let encodable: <<T as HasCompact>::Type as EncodeAsRef<'_, T>>::RefType =
        From::from(&t);
    Encoded(encodable.encode())
}

/// Format for describing accounts
// pub type Address<T> = <<T as System>::Lookup as StaticLookup>::Source;
/// Basic Extrinsic Type. Does not contain an ERA
pub type BasicExtrinsic<T> = UncheckedExtrinsic<<T as System>::Address, Encoded, AnySignature, <T as System>::SignedExtra >;
/// A block with OpaqueExtrinsic as extrinsic type
pub type SubstrateBlock<T> = SignedBlock<BlockT<<T as System>::Header, OpaqueExtrinsic>>;

// pub type BlockNumber<T> = NumberOrHex<<T as System>::BlockNumber>;

/// Sent from Substrate API to be committed into the Database
#[derive(Debug, PartialEq, Eq)]
pub enum Data<T: System> {
    Header(Header<T>),
    FinalizedHead(Header<T>),
    // Hash(T::Hash),
    Block(Block<T>),
    Storage(Storage<T>),
    Event(Event<T>),
    SyncProgress(usize),
}

// new types to allow implementing of traits
#[derive(Debug, PartialEq, Eq)]
pub struct Header<T: System> {
    inner: T::Header
}

impl<T: System> Header<T> {

    pub fn new(header: T::Header) -> Self {
        Self {
            inner: header
        }
    }

    pub fn inner(&self) -> &T::Header {
        &self.inner
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Block<T: System>{
    inner: SubstrateBlock<T>
}

impl<T: System> Block<T> {

    pub fn new(block: SubstrateBlock<T>) -> Self {
        Self {
            inner: block
        }
    }

    pub fn inner(&self) -> &SubstrateBlock<T> {
        &self.inner
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Storage<T: System>{
    data: StorageData,
    key_type: StorageKeyType,
    hash: T::Hash
}

impl<T: System> Storage<T> {

    pub fn new(data: StorageData, key_type: StorageKeyType, hash: T::Hash) -> Self {
        Self { data, key_type, hash }
    }

    pub fn data(&self) -> &StorageData {
        &self.data
    }
    pub fn key_type(&self) -> &StorageKeyType {
        &self.key_type
    }
    pub fn hash(&self) -> &T::Hash {
        &self.hash
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Event<T: System> {
    change_set: StorageChangeSet<T::Hash>
}

impl<T: System> Event<T> {

    pub fn new(change_set: StorageChangeSet<T::Hash>) -> Self {
        Self { change_set }
    }

    pub fn change_set(&self) -> &StorageChangeSet<T::Hash> {
        &self.change_set
    }
}
