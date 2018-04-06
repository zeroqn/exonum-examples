use exonum::{crypto::PublicKey, storage::{Fork, KeySetIndex, ListIndex, MapIndex, Snapshot}};

use models::{Chairperson, Proposal, Voter};

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
