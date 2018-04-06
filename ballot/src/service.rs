use exonum::{encoding, api::Api, blockchain::{ApiContext, Service, Transaction, TransactionSet},
             crypto::Hash, messages::RawTransaction, storage::Snapshot};
use iron::Handler;
use router::Router;

use constants::SERVICE_ID;
use transactions::BallotTransactions;
use api::BallotApi;

pub struct BallotService;

impl Service for BallotService {
    fn service_name(&self) -> &'static str {
        "ballot"
    }

    fn service_id(&self) -> u16 {
        SERVICE_ID
    }

    fn tx_from_raw(&self, raw: RawTransaction) -> Result<Box<Transaction>, encoding::Error> {
        let tx = BallotTransactions::tx_from_raw(raw)?;
        Ok(tx.into())
    }

    fn state_hash(&self, _: &Snapshot) -> Vec<Hash> {
        vec![]
    }

    fn public_api_handler(&self, ctx: &ApiContext) -> Option<Box<Handler>> {
        let mut router = Router::new();
        let api = BallotApi {
            channel: ctx.node_channel().clone(),
            blockchain: ctx.blockchain().clone(),
        };
        api.wire(&mut router);
        Some(Box::new(router))
    }
}
