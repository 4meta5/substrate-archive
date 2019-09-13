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

use srml_system::Trait;
use runtime_primitives::{
    OpaqueExtrinsic as UncheckedExtrinsic,
    generic::{Block as BlockT, SignedBlock},
    traits::{Header}
};
use runtime_support::Parameter;
use serde::de::DeserializeOwned;
use substrate_rpc_api::chain::number::NumberOrHex;
use substrate_primitives::storage::StorageChangeSet;


pub type BlockNumber<T> = NumberOrHex<<T as Trait>::BlockNumber>;
pub type Block<T> = SignedBlock<BlockT<<T as System>::Header, UncheckedExtrinsic>>;

#[derive(Debug, PartialEq, Eq)]
pub enum Payload<T: System> {
    FinalizedHead(<T as System>::Header),
    BlockNumber(T::BlockNumber),
    Header(<T as System>::Header),
    Block(Block<T>),
    Event(StorageChangeSet<T::Hash>),
    None,
}

/// Sent from Substrate API to be committed into the Database
#[derive(Debug, PartialEq, Eq)]
pub struct Data<T: System> {
    pub payload: Payload<T>
}

pub trait System: Trait + Sized + 'static {
    type Header: Parameter + Header<Number = Self::BlockNumber, Hash = Self::Hash> + DeserializeOwned;
}
