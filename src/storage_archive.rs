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


/// manages getting and storing Substrate State
pub struct Storage {
    /// Create new State Manager
    pub fn new() -> Self {
        unimplemented!()
    }

    /// Crawls any historical state and commits it to database
    pub async fn sync() -> Result<(), ArchiveError> {
        Ok(())
    }

    /// Verify if all state that can be stored (From Substrate Chain) is stored
    pub async fn verify() -> Result<bool, ArchiveError> {

        Ok(())
    }
}
