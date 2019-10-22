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

use failure::Error;
use substrate_archive::{
    Archive, System, Module,
    ExtractCall, SrmlExt, NotHandled,
    srml::{srml_system as system}
};
use polkadot_runtime::{Runtime as RuntimeT, Call};
use codec::{Encode, Decode, Input, Error as CodecError};


fn main() -> Result<(), Error> {
    env_logger::init();
    Archive::<Runtime>::new("ws://127.0.0.1:9944")?.run()?;
    Ok(())
}

// need to define Encode/Decode for Call New Type

// Passthrough traits (Boilerplate)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CallWrapper { inner: Call }
impl Encode for CallWrapper {
    fn encode(&self) -> Vec<u8> {
        self.inner.encode()
    }
}

impl Decode for CallWrapper {
    fn decode<I: Input>(input: &mut I) -> Result<Self, CodecError> {
        let decoded: Call = Decode::decode(input)?;
        Ok(CallWrapper {
            inner: decoded
        })
    }
}

// define all calls/inherents that you want tracked by the archive node
impl ExtractCall for CallWrapper {
    fn extract_call(&self) -> (Module, &dyn SrmlExt) {
        match &self.inner {
            Call::Timestamp(call) => {
                (Module::Timestamp, call)
            },
            Call::FinalityTracker(call) => {
                (Module::FinalityTracker, call)
            },
            Call::ImOnline(call) => {
                (Module::ImOnline, call)
            },
            Call::Babe(call) => {
                (Module::Babe, call)
            },
            Call::Staking(call) => {
                (Module::Staking, call)
            },
            Call::Session(call) => {
                (Module::Session, call)
            },
            Call::Grandpa(call) => {
                (Module::Grandpa, call)
            },
            Call::Treasury(call) => {
                (Module::Treasury, call)
            }
            c @ _ => {
                println!("{:?}", c);
                (Module::NotHandled, &NotHandled)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Runtime;
impl System for Runtime {
    type Call = CallWrapper;
    type Index = <RuntimeT as system::Trait>::Index;
    type BlockNumber = <RuntimeT as system::Trait>::BlockNumber;
    type Hash = <RuntimeT as system::Trait>::Hash;
    type Hashing = <RuntimeT as system::Trait>::Hashing;
    type AccountId = <RuntimeT as system::Trait>::AccountId;
    type Lookup = <RuntimeT as system::Trait>::Lookup;
    type Header = <RuntimeT as system::Trait>::Header;
    type Event = <RuntimeT as system::Trait>::Event;
    type SignedExtra = polkadot_runtime::SignedExtra;
}
