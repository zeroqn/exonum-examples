use exonum::{api::{Api, ApiError}, blockchain::{Blockchain, Transaction},
             crypto::{Hash, PublicKey}, encoding::serialize::FromHex,
             node::{ApiSender, TransactionSend}};
use iron::{headers::ContentType, modifiers::Header, prelude::*, status::Status};
use router::Router;
use serde_json;
use bodyparser;

use models::{Proposal, Voter};
use schema::BallotSchema;
use transactions::BallotTransactions;

macro_rules! post_handler {
    ($api: expr) => {{
        let api = $api.clone();
        move |req: &mut Request| api.post_transaction(req)
    }}
}

#[derive(Clone)]
pub struct BallotApi {
    pub channel: ApiSender,
    pub blockchain: Blockchain,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionResponse {
    pub tx_hash: Hash,
}

impl BallotApi {
    fn get_voter(&self, req: &mut Request) -> IronResult<Response> {
        let path = req.url.path();
        let voter_key = path.last().unwrap();
        let public_key = PublicKey::from_hex(voter_key).map_err(|e| {
            IronError::new(
                e,
                (
                    Status::BadRequest,
                    Header(ContentType::json()),
                    "\"Invalid request param: `pub_key`\"",
                ),
            )
        })?;

        let voter = {
            let snapshot = self.blockchain.snapshot();
            let schema = BallotSchema::new(snapshot);
            schema.voter(&public_key)
        };

        if let Some(voter) = voter {
            self.ok_response(&serde_json::to_value(voter).unwrap())
        } else {
            self.not_found_response(&serde_json::to_value("Voter not found").unwrap())
        }
    }

    fn get_voters(&self, _: &mut Request) -> IronResult<Response> {
        let snapshot = self.blockchain.snapshot();
        let schema = BallotSchema::new(snapshot);
        let idx = schema.voters();
        let voters: Vec<Voter> = idx.values().collect();

        self.ok_response(&serde_json::to_value(&voters).unwrap())
    }

    fn get_proposals(&self, _: &mut Request) -> IronResult<Response> {
        let snapshot = self.blockchain.snapshot();
        let schema = BallotSchema::new(snapshot);
        let idx = schema.proposals();
        let proposals: Vec<Proposal> = idx.iter().collect();

        self.ok_response(&serde_json::to_value(&proposals).unwrap())
    }

    fn get_proposal(&self, req: &mut Request) -> IronResult<Response> {
        let path = req.url.path();
        let proposal_id = path.last().unwrap().parse::<u64>().map_err(|e| {
            IronError::new(
                e,
                (
                    Status::BadRequest,
                    Header(ContentType::json()),
                    "\"Invalid request param: `proposal id`\"",
                ),
            )
        })?;

        let proposal = {
            let snapshot = self.blockchain.snapshot();
            let schema = BallotSchema::new(snapshot);
            schema.proposal(proposal_id)
        };

        if let Some(proposal) = proposal {
            self.ok_response(&serde_json::to_value(proposal).unwrap())
        } else {
            self.not_found_response(&serde_json::to_value("Proposal not found").unwrap())
        }
    }

    fn post_transaction(&self, req: &mut Request) -> IronResult<Response> {
        match req.get::<bodyparser::Struct<BallotTransactions>>() {
            Ok(Some(transaction)) => {
                let transaction: Box<Transaction> = transaction.into();
                let tx_hash = transaction.hash();
                self.channel.send(transaction).map_err(ApiError::from)?;
                let json = TransactionResponse { tx_hash };
                self.ok_response(&serde_json::to_value(&json).unwrap())
            }
            Ok(None) => Err(ApiError::BadRequest("Empty request body".into()))?,
            Err(e) => Err(ApiError::BadRequest(e.to_string()))?,
        }
    }
}

impl Api for BallotApi {
    fn wire(&self, router: &mut Router) {
        let post_create_voter = post_handler!(self);
        let get_voters = {
            let api = self.clone();
            move |req: &mut Request| api.get_voters(req)
        };
        let get_voter = {
            let api = self.clone();
            move |req: &mut Request| api.get_voter(req)
        };

        let post_new_proposals = post_handler!(self);
        let get_proposals = {
            let api = self.clone();
            move |req: &mut Request| api.get_proposals(req)
        };
        let get_proposal = {
            let api = self.clone();
            move |req: &mut Request| api.get_proposal(req)
        };

        router.post("/v1/voters", post_create_voter, "post_create_voter");
        router.get("/v1/voters", get_voters, "get_voters");
        router.get("/v1/voter/:pub_key", get_voter, "get_voter");

        router.post("/v1/proposals", post_new_proposals, "post_new_proposals");
        router.get("/v1/proposals", get_proposals, "get_proposals");
        router.get("/v1/proposals/:id", get_proposal, "get_proposal");
    }
}
