use std::borrow::Cow;
use std::ops::Deref;

use exonum::crypto::{self, CryptoHash, Hash, PublicKey, Signature};
use exonum::storage::{Fork, ProofListIndex, ProofMapIndex, Snapshot, StorageValue};
use serde_json::{self, Error as JsonError};

use transactions::{Ballot, Vote};

macro_rules! define_names {
    ($($name:ident => $value:expr;)+) => (
        $(const $name: &str = concat!("ballot.", $value);)*
    )
}

define_names! {
    BALLOTS => "ballots";
    PROPOSALS_HASHES => "proposals_hashes";
    VOTES => "votes";
}

lazy_static! {
    static ref NO_VOTE_BYTES: Vec<u8> = Vote::new_with_signature(
        &PublicKey::zero(),
        &Hash::zero(),
        0,
        "",
        &Signature::zero(),
    ).into_bytes();
}

/// json example:
/// {"proposals": [{"id": 1, subject: "lina", "description": "example"}]}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Proposal {
    id: u64,
    subject: String,
    description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProposalList {
    id: u64,
    deadline: u64,
    proposals: Vec<Proposal>,
}

impl ProposalList {
    pub fn try_serialize(&self) -> Result<Vec<u8>, JsonError> {
        serde_json::to_vec(self)
    }

    pub fn try_deserialize(serialized: &[u8]) -> Result<ProposalList, JsonError> {
        serde_json::from_slice::<ProposalList>(serialized)
    }

    pub fn has_duplicate_id(&self) -> bool {
        for proposal in self.proposals.iter() {
            // unique id only splits list into two parts
            let mut iter = self.proposals.split(|p| p.id == proposal.id);
            iter.next();
            iter.next();
            if !iter.next().is_none() {
                return true;
            }
        }
        false
    }

    pub fn contains(&self, id: u64, subject: &str) -> bool {
        for proposal in self.proposals.iter() {
            if proposal.id == id && proposal.subject == subject {
                return true;
            }
        }
        false
    }

    pub fn deadline(&self) -> u64 {
        self.deadline
    }
}

impl CryptoHash for ProposalList {
    fn hash(&self) -> Hash {
        let vec_bytes = self.try_serialize().unwrap();
        crypto::hash(&vec_bytes)
    }
}

impl StorageValue for ProposalList {
    fn into_bytes(self) -> Vec<u8> {
        self.try_serialize().unwrap()
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Self::try_deserialize(bytes.as_ref()).unwrap()
    }
}

encoding_struct! {
    struct BallotData {
        tx_ballot: Ballot,
        votes_history_hash: &Hash,
        num_voters: u64,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaybeVote(Option<Vote>);

impl MaybeVote {
    pub fn none() -> Self {
        MaybeVote(None)
    }

    pub fn some(vote: Vote) -> Self {
        MaybeVote(Some(vote))
    }
}

impl From<MaybeVote> for Option<Vote> {
    fn from(vote: MaybeVote) -> Option<Vote> {
        vote.0
    }
}

impl Deref for MaybeVote {
    type Target = Option<Vote>;

    fn deref(&self) -> &Option<Vote> {
        &self.0
    }
}

impl CryptoHash for MaybeVote {
    fn hash(&self) -> Hash {
        match self.0 {
            Some(ref vote) => vote.hash(),
            None => crypto::hash(&NO_VOTE_BYTES),
        }
    }
}

impl StorageValue for MaybeVote {
    fn into_bytes(self) -> Vec<u8> {
        match self.0 {
            Some(vote) => vote.into_bytes(),
            None => NO_VOTE_BYTES.clone(),
        }
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        if NO_VOTE_BYTES.as_slice().eq(bytes.as_ref()) {
            MaybeVote::none()
        } else {
            MaybeVote::some(Vote::from_bytes(bytes))
        }
    }
}

pub struct Schema<T> {
    view: T,
}

impl<T: AsRef<Snapshot>> Schema<T> {
    pub fn new(view: T) -> Self {
        Schema { view }
    }

    pub fn ballot_data_by_proposals_hash(&self) -> ProofMapIndex<&Snapshot, Hash, BallotData> {
        ProofMapIndex::new(BALLOTS, self.view.as_ref())
    }

    pub fn proposals_hash_by_ordinal(&self) -> ProofListIndex<&Snapshot, Hash> {
        ProofListIndex::new(PROPOSALS_HASHES, self.view.as_ref())
    }

    pub fn ballot(&self, proposals_hash: &Hash) -> Option<Ballot> {
        self.ballot_data_by_proposals_hash()
            .get(proposals_hash)?
            .tx_ballot()
            .into()
    }

    pub fn votes_by_proposals_hash(
        &self,
        proposals_hash: &Hash,
    ) -> ProofListIndex<&Snapshot, MaybeVote> {
        ProofListIndex::new_in_family(VOTES, proposals_hash, self.view.as_ref())
    }

    pub fn state_hash(&self) -> Vec<Hash> {
        vec![
            self.ballot_data_by_proposals_hash().merkle_root(),
            self.proposals_hash_by_ordinal().merkle_root(),
        ]
    }

    #[cfg_attr(feature = "cargo-clippy", allow(let_and_return))]
    pub fn votes(&self, proposals_hash: &Hash) -> Vec<Option<Vote>> {
        let votes = self.votes_by_proposals_hash(proposals_hash);
        let votes = votes.iter().map(MaybeVote::into).collect();
        votes
    }
}

impl<'a> Schema<&'a mut Fork> {
    pub(crate) fn ballot_data_by_proposals_hash_mut(
        &mut self,
    ) -> ProofMapIndex<&mut Fork, Hash, BallotData> {
        ProofMapIndex::new(BALLOTS, &mut self.view)
    }

    pub(crate) fn proposals_hash_by_ordinal_mut(&mut self) -> ProofListIndex<&mut Fork, Hash> {
        ProofListIndex::new(PROPOSALS_HASHES, &mut self.view)
    }

    pub(crate) fn votes_by_proposals_hash_mut(
        &mut self,
        proposals_hash: &Hash,
    ) -> ProofListIndex<&mut Fork, MaybeVote> {
        ProofListIndex::new_in_family(VOTES, proposals_hash, &mut self.view)
    }
}
