use exonum::{api::{Api, ApiError}, blockchain::{Blockchain, Transaction},
             crypto::{Hash, PublicKey}, encoding::serialize::FromHex,
             node::{ApiSender, TransactionSend}, storage::Snapshot};
use iron::{headers::ContentType, modifiers::Header, prelude::*, status::Status};
use router::Router;
use serde_json;
use bodyparser;

use models::{Voter, Voting};
use schema::BallotSchema;
use transactions::BallotTransactions;

macro_rules! handler {
    ($api: expr, $method: expr) => {{
        let api = $api.clone();
        move |req: &mut Request| api.invoke($method, req)
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

enum Method {
    Post,
    GetVoters,
    GetVoter,
    GetVotings,
    GetVoting,
    GetWinningProposal,
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

    fn get_votings(&self, _: &mut Request) -> IronResult<Response> {
        let schema = self.get_schema();
        let idx = schema.votings();
        let votings: Vec<Voting> = idx.iter().collect();

        self.ok_response(&serde_json::to_value(&votings).unwrap())
    }

    fn get_voting(&self, req: &mut Request) -> IronResult<Response> {
        let path = req.url.path();
        let voting_id = path.last().unwrap().parse::<u64>().map_err(|e| {
            IronError::new(
                e,
                (
                    Status::BadRequest,
                    Header(ContentType::json()),
                    "\"Invalid request param: `proposal id`\"",
                ),
            )
        })?;

        let schema = self.get_schema();
        if let Some(voting) = schema.voting(voting_id) {
            self.ok_response(&serde_json::to_value(&voting).unwrap())
        } else {
            self.not_found_response(&serde_json::to_value("Voting not found").unwrap())
        }
    }

    fn get_winning_proposal(&self, req: &mut Request) -> IronResult<Response> {
        let path = req.url.path();
        let voting_id = path.last().unwrap().parse::<u64>().map_err(|e| {
            IronError::new(
                e,
                (
                    Status::BadRequest,
                    Header(ContentType::json()),
                    "\"Invalid request param: `proposal id`\"",
                ),
            )
        })?;

        let schema = self.get_schema();
        match schema.voting(voting_id) {
            Some(ref voting) if voting.has_done() => {
                let proposals = voting.proposals();
                let mut winning_proposal = proposals.last().unwrap();

                for proposal in proposals.iter() {
                    if proposal.vote_count() > winning_proposal.vote_count() {
                        winning_proposal = proposal;
                    }
                }

                self.ok_response(&serde_json::to_value(&winning_proposal).unwrap())
            }
            Some(_) => self.ok_response(&serde_json::to_value("Voting not done yet").unwrap()),
            None => self.not_found_response(&serde_json::to_value("Voting not found").unwrap()),
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

    fn invoke(&self, method: Method, req: &mut Request) -> IronResult<Response> {
        match method {
            Method::Post => self.post_transaction(req),
            Method::GetVoter => self.get_voter(req),
            Method::GetVoters => self.get_voters(req),
            Method::GetVotings => self.get_votings(req),
            Method::GetVoting => self.get_voting(req),
            Method::GetWinningProposal => self.get_winning_proposal(req),
        }
    }

    fn get_schema(&self) -> BallotSchema<Box<Snapshot>> {
        let snapshot = self.blockchain.snapshot();
        BallotSchema::new(snapshot)
    }
}

impl Api for BallotApi {
    fn wire(&self, router: &mut Router) {
        router.post(
            "/v1/voters",
            handler!(self, Method::Post),
            "post_create_voter",
        );
        router.get(
            "/v1/voters",
            handler!(self, Method::GetVoters),
            "get_voters",
        );
        router.get(
            "/v1/voter/:pub_key",
            handler!(self, Method::GetVoter),
            "get_voter",
        );

        router.post(
            "/v1/votings",
            handler!(self, Method::Post),
            "post_new_votings",
        );
        router.get(
            "/v1/votings",
            handler!(self, Method::GetVotings),
            "get_votings",
        );
        router.get(
            "/v1/votings/:id",
            handler!(self, Method::GetVoting),
            "get_voting",
        );
        router.get(
            "/v1/votings/:id/winner",
            handler!(self, Method::GetWinningProposal),
            "get_winning_proposal",
        );
    }
}
