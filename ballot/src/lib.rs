extern crate bodyparser;
#[macro_use]
extern crate exonum;
#[macro_use]
extern crate failure;
extern crate iron;
extern crate router;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use exonum::api::{Api, ApiError};
use exonum::blockchain::{ApiContext, Blockchain, ExecutionError, ExecutionResult, Service,
                         Transaction, TransactionSet};
use exonum::crypto::{Hash, PublicKey};
use exonum::encoding;
use exonum::encoding::serialize::FromHex;
use exonum::messages::{Message, RawTransaction};
use exonum::node::{ApiSender, TransactionSend};
use exonum::storage::{Fork, KeySetIndex, ListIndex, MapIndex, Snapshot};
use iron::Handler;
use iron::headers::ContentType;
use iron::modifiers::Header;
use iron::prelude::*;
use iron::status::Status;
use router::Router;

pub mod constants;

use constants::{INIT_WEIGHT, MAX_PROPOSALS, SERVICE_ID};

macro_rules! post_handler {
    ($api: expr) => {{
        let api = $api.clone();
        move |req: &mut Request| api.post_transaction(req)
    }}
}

encoding_struct! {
    struct Voter {
        pub_key: &PublicKey,
        name: &str,
        weight: u16,
        is_active: bool,
    }
}

encoding_struct! {
    struct Chairperson {
        pub_key: &PublicKey,
        name: &str,
    }
}

encoding_struct! {
    struct Proposal {
        subject: &str,
        vote_count: u16,
    }
}

encoding_struct! {
    struct NewProposal {
        subject: &str,
    }
}

pub struct BallotSchema<T> {
    view: T,
}

impl<T: AsRef<Snapshot>> BallotSchema<T> {
    pub fn new(view: T) -> Self {
        BallotSchema { view }
    }

    pub fn voters(&self) -> MapIndex<&Snapshot, PublicKey, Voter> {
        MapIndex::new("ballot.voters", self.view.as_ref())
    }

    pub fn voter(&self, pub_key: &PublicKey) -> Option<Voter> {
        self.voters().get(pub_key)
    }

    pub fn chairperson(&self) -> Option<Chairperson> {
        let chairperson: ListIndex<&Snapshot, Chairperson> =
            ListIndex::new("ballot.chairperson", self.view.as_ref());
        chairperson.last()
    }

    pub fn proposals(&self) -> ListIndex<&Snapshot, Proposal> {
        ListIndex::new("ballot.proposals", self.view.as_ref())
    }

    pub fn proposal(&self, proposal_id: u64) -> Option<Proposal> {
        self.proposals().get(proposal_id)
    }

    pub fn voted_voters(&self) -> KeySetIndex<&Snapshot, PublicKey> {
        KeySetIndex::new("ballot.votedvoters", self.view.as_ref())
    }
}

impl<'a> BallotSchema<&'a mut Fork> {
    pub fn voters_mut(&mut self) -> MapIndex<&mut Fork, PublicKey, Voter> {
        MapIndex::new("ballot.voters", &mut self.view)
    }

    pub fn voted_voters_mut(&mut self) -> KeySetIndex<&mut Fork, PublicKey> {
        KeySetIndex::new("ballot.votedvoters", &mut self.view)
    }

    pub fn proposals_mut(&mut self) -> ListIndex<&mut Fork, Proposal> {
        ListIndex::new("ballot.proposals", &mut self.view)
    }

    pub fn set_chairperson(&mut self, new_one: Chairperson) {
        let mut chairperson: ListIndex<&mut Fork, Chairperson> =
            ListIndex::new("ballot.chairperson", &mut self.view);
        chairperson.clear();
        chairperson.push(new_one);
    }
}

transactions!{
    BallotTransactions {
        const SERVICE_ID = SERVICE_ID;

        struct TxCreateVoter {
            pub_key: &PublicKey,
            name: &str,
        }

        struct TxANewChairperson {
            pub_key: &PublicKey,
            new_chairperson_pubkey: &PublicKey,
        }

        struct TxASetVoterActiveState {
            pub_key: &PublicKey,
            voter_pubkey: &PublicKey,
            active_state: bool,
        }

        struct TxNewProposals {
            pub_key: &PublicKey,
            new_proposals: Vec<NewProposal>,
        }

        struct TxVoteProposal {
            pub_key: &PublicKey,
            id: u16,
        }
    }
}

#[derive(Debug, Fail)]
#[repr(u8)]
pub enum Error {
    #[fail(display = "Voter already exists")]
    VoterAlreadyExists = 0,
    #[fail(display = "Voter permission required")]
    VoterPermissionRequired = 1,
    #[fail(display = "Excess max proposals")]
    ExcessMaxProposals = 2,
    #[fail(display = "Proposal none exists")]
    ProposalNoneExists = 3,
    #[fail(display = "Voter already voted")]
    VoterAlreadyVoted = 4,
    #[fail(display = "Chairperson permission required")]
    ChairpersonPermissionRequired = 5,
    #[fail(display = "Voter none exists")]
    VoterNoneExists = 6,
    #[fail(display = "Voter inactive")]
    VoterInactive = 7,
}

impl From<Error> for ExecutionError {
    fn from(value: Error) -> ExecutionError {
        ExecutionError::new(value as u8)
    }
}

impl Transaction for TxCreateVoter {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        let mut schema = BallotSchema::new(view);
        if schema.voter(self.pub_key()).is_none() {
            let voter = Voter::new(self.pub_key(), self.name(), INIT_WEIGHT, true);
            println!("Create the voter: {:?}", voter);
            schema.voters_mut().put(self.pub_key(), voter);

            if schema.chairperson().is_none() {
                let chairperson = Chairperson::new(self.pub_key(), self.name());
                println!("New chair person: {:?}", chairperson);
                schema.set_chairperson(chairperson);
            }

            Ok(())
        } else {
            Err(Error::VoterAlreadyExists)?
        }
    }
}

impl Transaction for TxANewChairperson {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        let mut schema = BallotSchema::new(view);
        has_chairperson_perm(&schema, self.pub_key())?;

        let voter = schema.voter(self.new_chairperson_pubkey());
        match voter {
            Some(ref voter) if voter.is_active() => {
                let new_chairperson = Chairperson::new(self.new_chairperson_pubkey(), voter.name());
                println!("new chair person: {:?}", new_chairperson);
                schema.set_chairperson(new_chairperson);

                Ok(())
            }
            Some(_) => Err(Error::VoterInactive)?,
            None => Err(Error::VoterNoneExists)?,
        }
    }
}

impl Transaction for TxASetVoterActiveState {
    fn verify(&self) -> bool {
        self.pub_key() != self.voter_pubkey() && self.verify_signature(self.pub_key())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        let mut schema = BallotSchema::new(view);
        has_chairperson_perm(&schema, self.pub_key())?;

        let voter = schema.voter(self.voter_pubkey());
        if let Some(voter) = voter {
            println!("Change voter {:?} active state", voter);
            let updated_voter = Voter::new(
                voter.pub_key(),
                voter.name(),
                voter.weight(),
                self.active_state(),
            );

            schema.voters_mut().put(voter.pub_key(), updated_voter);

            Ok(())
        } else {
            Err(Error::VoterNoneExists)?
        }
    }
}

impl Transaction for TxNewProposals {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        if self.new_proposals().len() > MAX_PROPOSALS as usize {
            Err(Error::ExcessMaxProposals)?
        }

        let mut schema = BallotSchema::new(view);
        // TODO: maybe add a chairperson to create new ballot
        has_voter_perm(&schema, self.pub_key())?;

        println!("Create new ballot!");
        let mut proposals = schema.proposals_mut();
        for proposal in self.new_proposals() {
            let proposal = Proposal::new(proposal.subject(), 0);
            println!("Add proposal: {:?}", proposal);
            proposals.push(proposal);
        }
        Ok(())
    }
}

impl Transaction for TxVoteProposal {
    fn verify(&self) -> bool {
        self.verify_signature(self.pub_key())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        let mut schema = BallotSchema::new(view);
        has_voter_perm(&schema, self.pub_key())?;

        if schema.voted_voters().contains(self.pub_key()) {
            Err(Error::VoterAlreadyVoted)?
        }

        let voter_weight = {
            let voter = schema.voter(self.pub_key()).unwrap();
            voter.weight()
        };

        // Update proposal's vote count
        {
            let mut proposals = schema.proposals_mut();
            let proposal = proposals.get(self.id().into());
            if let Some(proposal) = proposal {
                let updated =
                    Proposal::new(proposal.subject(), proposal.vote_count() + voter_weight);
                proposals.set(self.id().into(), updated);
            } else {
                Err(Error::ProposalNoneExists)?
            }
        }

        // Mark voter as voted
        {
            let mut voted_voters = schema.voted_voters_mut();
            voted_voters.insert(self.pub_key().to_owned());
        }

        Ok(())
    }
}

fn has_chairperson_perm(
    schema: &BallotSchema<&mut Fork>,
    pub_key: &PublicKey,
) -> Result<Chairperson, Error> {
    let chairperson = schema.chairperson();
    debug_assert!(chairperson.is_some());
    let chairperson = chairperson.unwrap();
    println!("Current chairperson: {:?}", chairperson);

    if pub_key != chairperson.pub_key() {
        Err(Error::ChairpersonPermissionRequired)?
    }
    Ok(chairperson)
}

fn has_voter_perm(schema: &BallotSchema<&mut Fork>, pub_key: &PublicKey) -> Result<Voter, Error> {
    let voter = schema.voter(pub_key);
    match voter {
        Some(ref voter) if voter.is_active() => Ok(voter.to_owned()),
        Some(_) => Err(Error::VoterInactive),
        None => Err(Error::VoterPermissionRequired),
    }
}

#[derive(Clone)]
struct BallotApi {
    channel: ApiSender,
    blockchain: Blockchain,
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
