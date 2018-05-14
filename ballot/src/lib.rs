extern crate bodyparser;
#[macro_use]
extern crate exonum;
#[macro_use]
extern crate failure;
extern crate iron;
#[macro_use]
extern crate lazy_static;
extern crate router;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

#[cfg(test)]
#[macro_use]
extern crate exonum_testkit;

const SERVICE_ID: u16 = 1;
const SERVICE_NAME: &'static str = "ballot";

pub mod schema;
pub mod transactions;
pub mod error;
pub mod api;
#[cfg(test)]
mod tests;

use exonum::encoding;
use exonum::api::Api;
use exonum::blockchain::{ApiContext, Service, Transaction, TransactionSet};
use exonum::crypto::Hash;
use exonum::messages::RawTransaction;
use exonum::storage::Snapshot;
use iron::Handler;
use router::Router;

use schema::Schema;
use transactions::Transactions as BallotTransactions;
use api as BallotApi;

pub struct BallotService;

impl Service for BallotService {
    fn service_name(&self) -> &'static str {
        SERVICE_NAME
    }

    fn service_id(&self) -> u16 {
        SERVICE_ID
    }

    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<Transaction>, encoding::Error> {
        let tx = BallotTransactions::tx_from_raw(raw)?;
        Ok(tx.into())
    }

    fn state_hash(&self, snapshot: &Snapshot) -> Vec<Hash> {
        let schema = Schema::new(snapshot);
        schema.state_hash()
    }

    fn private_api_handler(&self, ctx: &ApiContext) -> Option<Box<Handler>> {
        let mut router = Router::new();
        let api = BallotApi::PrivateApi::new(ctx);
        api.wire(&mut router);
        Some(Box::new(router))
    }
}
