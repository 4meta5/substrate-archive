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

use crate::{ types::System, database::Database, rpc::Rpc, error::Error };
use std::sync::Arc;

/// manages getting and storing Substrate State
pub struct StorageArchive<T: System> {
    db: Arc<Database>,
    rpc: Arc<Rpc<T>>
}

impl<T> StorageArchive<T> where T: System {
    /// Create new State Manager
    pub fn new(db: Arc<Database>, rpc: Arc<Rpc<T>>) -> Self {
        Self { db, rpc }
    }

    pub async fn run(&mut self) {
        ()
    }

    /// Crawls any historical state and commits it to database
    async fn sync() -> Result<(), Error> {
        Ok(())
    }

    /// Verify if all state that can be stored (From Substrate Chain) is stored
    pub async fn verify() -> Result<bool, Error> {

        Ok(true)
    }
}
