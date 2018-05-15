use exonum::api::{Api as ExonumApi, ApiError};
use exonum::blockchain::{ApiContext, Blockchain};
use exonum::crypto::{CryptoHash, Hash, PublicKey, SecretKey};
use exonum::node::{ApiSender, TransactionSend};
use exonum::storage::StorageValue;

use iron::prelude::*;

use router::Router;
use serde_json;
use bodyparser;

use schema::ProposalList;
use transactions::{Ballot, Vote};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BallotResponse {
    pub tx_hash: Hash,
    pub proposals_hash: Hash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRequest {
    pub proposal_id: u64,
    pub proposal_subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VoteResponse {
    pub tx_hash: Hash,
}

#[derive(Clone)]
pub struct PrivateApi {
    channel: ApiSender,
    service_keys: (PublicKey, SecretKey),
}

#[derive(Clone)]
pub struct PublicApi {
    blockchain: Blockchain,
}

impl PublicApi {
    pub fn new(context: &ApiContext) -> Self {
        PublicApi {
            blockchain: context.blockchain().clone(),
        }
    }
}

impl PrivateApi {
    pub fn new(context: &ApiContext) -> Self {
        PrivateApi {
            channel: context.node_channel().clone(),
            service_keys: (*context.public_key(), context.secret_key().clone()),
        }
    }

    fn handle_ballot(self, router: &mut Router) {
        let post_ballot = move |req: &mut Request| -> IronResult<Response> {
            let proposals = match req.get::<bodyparser::Struct<ProposalList>>() {
                Ok(Some(proposals)) => proposals,
                Ok(None) => Err(ApiError::BadRequest("Empty request body".into()))?,
                Err(e) => Err(ApiError::BadRequest(e.to_string()))?,
            };

            let proposals_hash = proposals.hash();
            let propose = Ballot::new(
                &self.service_keys.0,
                ::std::str::from_utf8(proposals.into_bytes().as_slice()).unwrap(),
                &self.service_keys.1,
            );
            let tx_hash = propose.hash();

            self.channel.send(propose.into()).map_err(ApiError::from)?;

            let response = BallotResponse {
                tx_hash,
                proposals_hash,
            };
            self.ok_response(&serde_json::to_value(response).unwrap())
        };

        router.post("/v1/ballots", post_ballot, "post_ballot");
    }

    fn handle_vote(self, router: &mut Router) {
        let post_vote = move |req: &mut Request| -> IronResult<Response> {
            let proposals_hash = self.url_fragment::<Hash>(req, "hash")?;
            let vote_req = match req.get::<bodyparser::Struct<VoteRequest>>() {
                Ok(Some(vote_req)) => vote_req,
                Ok(None) => Err(ApiError::BadRequest("Empty request body".into()))?,
                Err(e) => Err(ApiError::BadRequest(e.to_string()))?,
            };

            let vote = Vote::new(
                &self.service_keys.0,
                &proposals_hash,
                vote_req.proposal_id,
                &vote_req.proposal_subject,
                &self.service_keys.1,
            );
            let tx_hash = vote.hash();

            self.channel.send(vote.into()).map_err(ApiError::from)?;

            let response = VoteResponse { tx_hash };
            self.ok_response(&serde_json::to_value(response).unwrap())
        };

        router.post("/v1/ballot/:hash/postvote", post_vote, "post_vote");
    }
}

impl ExonumApi for PrivateApi {
    fn wire(&self, router: &mut Router) {
        self.clone().handle_ballot(router);
        self.clone().handle_vote(router);
    }
}
