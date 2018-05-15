use exonum::api::{Api as ExonumApi, ApiError};
use exonum::blockchain::{ApiContext, Blockchain};
use exonum::crypto::{CryptoHash, Hash, PublicKey, SecretKey};
use exonum::node::{ApiSender, TransactionSend};
use exonum::storage::StorageValue;

use iron::prelude::*;

use router::Router;
use serde_json;
use bodyparser;

use schema::{BallotData, ProposalList, Schema};
use transactions::{Ballot, Vote};

pub type VotesInfo = Option<Vec<Option<Vote>>>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BallotHashInfo {
    pub ballot: Option<BallotData>,
    pub hash: Option<Hash>,
    pub proposals: Option<ProposalList>,
    pub proposals_hash: Hash,
    pub votes: VotesInfo,
}

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

    fn ballots(&self, offset: u64, limit: usize) -> Vec<BallotHashInfo> {
        let schema = Schema::new(self.blockchain.snapshot());
        let proposals_hashs = schema.proposals_hash_by_ordinal();
        let ballots = proposals_hashs
            .iter_from(offset)
            .take(limit)
            .map(|hash| self.ballot_with_proofs(&hash))
            .collect();
        ballots
    }

    fn ballot_with_proofs(&self, proposals_hash: &Hash) -> BallotHashInfo {
        let schema = Schema::new(self.blockchain.snapshot());
        if let Some(ballot_data) = schema.ballot_data_by_proposals_hash().get(proposals_hash) {
            let hash = Some(ballot_data.hash());
            let tx_ballot = ballot_data.tx_ballot();
            let proposals_str = tx_ballot.proposals();
            let proposals = Some(ProposalList::try_deserialize(proposals_str.as_bytes()).unwrap());
            let votes = Some(schema.votes(proposals_hash));
            BallotHashInfo {
                ballot: Some(ballot_data),
                hash,
                proposals,
                proposals_hash: *proposals_hash,
                votes,
            }
        } else {
            BallotHashInfo {
                ballot: None,
                hash: None,
                proposals: None,
                proposals_hash: *proposals_hash,
                votes: None,
            }
        }
    }

    fn votes_for_ballot(&self, proposals_hash: &Hash) -> VotesInfo {
        let schema = Schema::new(self.blockchain.snapshot());
        if schema
            .ballot_data_by_proposals_hash()
            .contains(proposals_hash)
        {
            Some(schema.votes(proposals_hash))
        } else {
            None
        }
    }

    fn handle_range_ballots(self, router: &mut Router) {
        let range_ballots = move |req: &mut Request| -> IronResult<Response> {
            let limit = self.required_param::<usize>(req, "limit")?;
            let offset = self.required_param::<u64>(req, "offset")?;
            let ballots = self.ballots(offset, limit);
            self.ok_response(&serde_json::to_value(ballots).unwrap())
        };

        router.get("/v1/ballots", range_ballots, "range_ballots");
    }

    fn handle_ballot_by_hash(self, router: &mut Router) {
        let ballot_by_hash = move |req: &mut Request| -> IronResult<Response> {
            let proposals_hash = self.url_fragment::<Hash>(req, "hash")?;
            let ballot_hash_info = self.ballot_with_proofs(&proposals_hash);
            self.ok_response(&serde_json::to_value(ballot_hash_info).unwrap())
        };

        router.get("/v1/ballots/:hash", ballot_by_hash, "ballot_by_hash");
    }

    fn handle_votes_for_ballot(self, router: &mut Router) {
        let votes_for_ballot = move |req: &mut Request| -> IronResult<Response> {
            let proposals_hash = self.url_fragment::<Hash>(req, "hash")?;
            let votes = self.votes_for_ballot(&proposals_hash);
            self.ok_response(&serde_json::to_value(votes).unwrap())
        };

        router.get(
            "/v1/ballots/:hash/votes",
            votes_for_ballot,
            "votes_for_ballot",
        );
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

        router.post("/v1/ballots/:hash/postvote", post_vote, "post_vote");
    }
}

impl ExonumApi for PublicApi {
    fn wire(&self, router: &mut Router) {
        self.clone().handle_range_ballots(router);
        self.clone().handle_ballot_by_hash(router);
        self.clone().handle_votes_for_ballot(router);
    }
}

impl ExonumApi for PrivateApi {
    fn wire(&self, router: &mut Router) {
        self.clone().handle_ballot(router);
        self.clone().handle_vote(router);
    }
}
