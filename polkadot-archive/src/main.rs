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

//! Specify types for a specific Blockchain -- E.G Kusama/Polkadot and run the archive node with these types

//use substrate_subxt::srml::{balances::Balances, contracts::Contracts, system::System};
use substrate_archive::types::System;
use runtime_primitives::{
    OpaqueExtrinsic as UncheckedExtrinsic,
    generic::{Era, SignedBlock},
    traits::StaticLookup
};
use node_primitives::{Hash, Header, Block}; // Block == Block<Header, UncheckedExtrinsic>
use polkadot_runtime::Runtime;


fn main() {
    substrate_archive::run::<Runtime>();
}

impl System for Runtime {
    type Header = Runtime::Header;
}
