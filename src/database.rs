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

//! IO for the PostgreSQL database connected to Substrate Archive Node

pub mod models;
pub mod schema;
pub mod db_middleware;

use log::*;
use futures::future::{self, Future};
use diesel::{prelude::*, pg::PgConnection, sql_types::BigInt};
use codec::{Encode, Decode};
use runtime_primitives::traits::Header;
use dotenv::dotenv;
use chrono::offset::{Utc, TimeZone};
use runtime_primitives::{
    traits::{Block as BlockTrait, Extrinsic},
    OpaqueExtrinsic, generic::UncheckedExtrinsic
};
use substrate_subxt::system::System;

use std::{
    env,
    convert::TryFrom
};

use crate::{
    error::Error as ArchiveError,
    types::{Data, Block, Storage, BasicExtrinsic},
    database::{
        models::{InsertBlock, InsertInherentOwned},
        schema::{blocks, inherents},
        db_middleware::AsyncDiesel
    },
    queries
};

pub type DbFuture = Box<dyn Future<Item = (), Error = ArchiveError> + Send >;

pub trait Insert {
    type Error;
    fn insert(&self, db: AsyncDiesel<PgConnection>) -> Result<DbFuture, Self::Error>;
}

impl<T> Insert for Data<T> where T: System + 'static {
    type Error = ArchiveError;
    fn insert(&self, db: AsyncDiesel<PgConnection>) -> Result<DbFuture, ArchiveError> {
        match self {
            Data::Block(block) => {
                block.insert(db)
            },
            Data::Storage(storage) => {
                storage.insert(db)
            },
            _=> {
                Err(ArchiveError::UnhandledDataType)
            }
        }
    }
}

/// Database object which communicates with Diesel in a (psuedo)asyncronous way
/// via `AsyncDiesel`
pub struct Database {
    db: AsyncDiesel<PgConnection>,
}

impl Database {

    /// Connect to the database
    pub fn new() -> Result<Self, ArchiveError> {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL")?;
        let db = AsyncDiesel::new(&database_url)?;
        Ok(Self { db })
    }

    pub fn insert<T>(&self, data: &Data<T>) -> DbFuture
    where
        T: System + 'static
    {
        match data.insert(self.db.clone()) {
            Ok(v) => v,
            Err(e) => Box::new(future::err(e)),
        }
    }

    pub fn query_missing_blocks(&self)
                          -> impl Future<Item = Vec<u64>, Error = ArchiveError>
    {
        #[derive(QueryableByName, PartialEq, Debug)]
        pub struct Blocks {
            #[column_name = "generate_series"]
            #[sql_type = "BigInt"]
            block_num: i64
        };

        self.db.run(move |conn| {
            let blocks: Vec<Blocks> = queries::missing_blocks().load(&conn)?;
            Ok(blocks
                .iter()
                .map(|b| u64::try_from(b.block_num).expect("Block number should never be negative; qed"))
               .collect::<Vec<u64>>())
        })
    }
}

impl<T> Insert for Storage<T>
where
    T: System,
{
    type Error = ArchiveError;

    fn insert(&self, db: AsyncDiesel<PgConnection>) -> Result<DbFuture, ArchiveError> {
        use self::schema::blocks::dsl::{blocks, time};
        let (data, key_type, hash) = (self.data(), self.key_type(), self.hash().clone());
        let unix_time: i64 = Decode::decode(&mut data.0.as_slice())?;
        let date_time = Utc.timestamp_millis(unix_time); // panics if time is incorrect
        let fut = db.run(move |conn| {
            diesel::update(blocks.find(hash.as_ref()))
                .set(time.eq(Some(&date_time)))
                .execute(&conn)
                .map_err(|e| e.into())
        }).map(|_| ());
        Ok(Box::new(fut))
    }
}

impl<T> Insert for Block<T>
where
    T: System + 'static
{
    type Error = ArchiveError;

    fn insert(&self, db: AsyncDiesel<PgConnection>) -> Result<DbFuture, ArchiveError> {
        let block = self.inner().block.clone();
        info!("HASH: {:X?}", block.header.hash().as_ref());
        let extrinsics_fut = insert_extrinsics::<T>(block.extrinsics(),
                                                    &block.header,
                                                    &db)?;
        let fut = db.run(move |conn| {
            trace!("Inserting Block: {:?}", block.clone());
            diesel::insert_into(blocks::table)
                .values( InsertBlock {
                    parent_hash: block.header.parent_hash().as_ref(),
                    hash: block.header.hash().as_ref(),
                    block_num: &(*block.header.number()).into(),
                    state_root: block.header.state_root().as_ref(),
                    extrinsics_root: block.header.extrinsics_root().as_ref(),
                    time: None
                })
                .execute(&conn)
                .map_err(|e| e.into())
        }).and_then(move |res| {
            extrinsics_fut
        }).map(|_| ());
        Ok(Box::new(fut))
    }
}

fn insert_extrinsics<T>(
    extrinsics: &[OpaqueExtrinsic],
    header: &T::Header,
    db: &AsyncDiesel<PgConnection>,
) -> Result<DbFuture, ArchiveError>
where
    T: System + 'static
{
    let values: Vec<InsertInherentOwned> = extrinsics
        .iter()
        // enumerate is used here to preserve order/index of extrinsics
        .enumerate()
        .map(|(idx, x)| Ok(( idx, UncheckedExtrinsic::decode(&mut x.encode().as_slice())? )))
        .collect::<Vec<Result<(usize, BasicExtrinsic<T>), ArchiveError>>>()
        .into_iter()
        // we don't want to skip over _all_ extrinsics if decoding one extrinsic does not work
        .filter_map(|x: Result<(usize, BasicExtrinsic<T>), _>| {
            match x {
                Ok(v) => {
                    if let Some(b) = v.1.is_signed() {
                        if b {
                            // will be inserted as signed_extrinsic
                            None
                        } else {
                            Some((v.0, v.1))
                        }
                    } else {
                        Some((v.0, v.1))
                    }
                },
                Err(e) => {
                    error!("{:?}", e);
                    None
                }
            }
        })
        // .map(|(idx, decoded)| {
            /*
            let (module, call) = decoded.function.extract_call();
            let index: i32 = i32::try_from(idx)?;
            let res = call.function();
            if res.is_err() {
                info!("Call : {:?}", decoded);
            }
            let (fn_name, params) = res?;
            Ok(InsertInherentOwned {
                hash: header.hash().as_ref().to_vec(),
                block_num: (*header.number()).into(),
                module: module.to_string(),
                call: fn_name,
                parameters: Some(params),
                success: true, // TODO: Success is not always true
                in_index: index
            })
            */
        // })
        .collect::<Vec<BasicExtrinsic<T>>, ArchiveError>();
    println!("{:?}", values);
/*
    let fut = db.run(move |conn| {
        trace!("Inserting Extrinsics: {:?}", values);
        diesel::insert_into(inherents::table)
            .values(&values)
            .execute(&conn)
            .map_err(|e| e.into())
    }).map(|_| ());
*/
    Ok(Box::new(future::ok(())))
}


#[cfg(test)]
mod tests {
    //! Must be connected to a local database
    use super::*;




}
