use exonum::{crypto::PublicKey, storage::{Fork, KeySetIndex, ListIndex, MapIndex, Snapshot}};

use models::{Chairperson, Voter, Voting};

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

    pub fn votings(&self) -> ListIndex<&Snapshot, Voting> {
        ListIndex::new("ballot.votings", self.view.as_ref())
    }

    pub fn voting(&self, voting_id: u64) -> Option<Voting> {
        self.votings().get(voting_id)
    }

    pub fn has_voted(&self, voting_id: u64, voter_pubkey: &PublicKey) -> bool {
        let voted_voters: KeySetIndex<&Snapshot, PublicKey> = KeySetIndex::new(
            "ballot.hasvoted.".to_owned() + &voting_id.to_string(),
            self.view.as_ref(),
        );
        voted_voters.contains(voter_pubkey)
    }
}

impl<'a> BallotSchema<&'a mut Fork> {
    pub fn voters_mut(&mut self) -> MapIndex<&mut Fork, PublicKey, Voter> {
        MapIndex::new("ballot.voters", &mut self.view)
    }

    pub fn set_chairperson(&mut self, new_one: Chairperson) {
        let mut chairperson: ListIndex<&mut Fork, Chairperson> =
            ListIndex::new("ballot.chairperson", &mut self.view);
        chairperson.clear();
        chairperson.push(new_one);
    }

    pub fn votings_mut(&mut self) -> ListIndex<&mut Fork, Voting> {
        ListIndex::new("ballot.votings", &mut self.view)
    }

    pub fn mark_voted(&mut self, voting_id: u64, voter_pubkey: &PublicKey) {
        let mut voted_voters: KeySetIndex<&mut Fork, PublicKey> = KeySetIndex::new(
            "ballot.hasvoted.".to_owned() + &voting_id.to_string(),
            &mut self.view,
        );
        voted_voters.insert(voter_pubkey.to_owned());
    }
}
