use std::time::{Duration as StdDuration, SystemTime};
use std::ops::Add;

use exonum::crypto::PublicKey;

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

encoding_struct!  {
    struct VoterPubKey {
        key: &PublicKey,
    }
}

encoding_struct! {
    struct Duration {
        seconds: u64,
    }
}

encoding_struct!{
    struct TimeLimit {
        deadline: SystemTime,
        start_time: SystemTime,
        duration: Duration,
    }
}

encoding_struct!{
    struct Voting {
        id: u64,
        proposals: Vec<Proposal>,
        voted_voters: Vec<VoterPubKey>,

        time_limit: TimeLimit,
    }
}

impl TimeLimit {
    pub fn new_limit(duration: u64) -> Self {
        let start_time = SystemTime::now();
        TimeLimit::new(
            start_time.clone().add(StdDuration::new(duration, 0)),
            start_time,
            Duration::new(duration),
        )
    }

    pub fn within_limit(&self) -> bool {
        SystemTime::now() <= self.deadline()
    }
}

impl Voting {
    pub fn new_with_limit(id: u64, proposals: Vec<Proposal>, duration_in_secs: u64) -> Self {
        Voting::new(
            id,
            proposals,
            vec![],
            TimeLimit::new_limit(duration_in_secs),
        )
    }

    pub fn vote(
        self,
        proposal_id: usize,
        voter_pubkey: &PublicKey,
        voter_weight: u16,
    ) -> Option<Voting> {
        let mut proposals = self.proposals();
        let mut voted_voters = self.voted_voters();

        let updated_proposal = {
            let proposal = proposals.get(proposal_id)?;
            Proposal::new(proposal.subject(), proposal.vote_count() + voter_weight)
        };
        proposals[proposal_id] = updated_proposal;
        voted_voters.push(VoterPubKey::new(voter_pubkey));

        Some(Voting::new(
            self.id(),
            proposals,
            voted_voters,
            self.time_limit(),
        ))
    }

    pub fn has_done(&self) -> bool {
        !self.time_limit().within_limit()
    }

    pub fn has_voted(&self, voter_pubkey: &PublicKey) -> bool {
        self.voted_voters()
            .contains(&VoterPubKey::new(voter_pubkey))
    }

    pub fn get_proposal(&self, proposal_id: usize) -> Option<Proposal> {
        self.proposals().get(proposal_id).map(|p| p.to_owned())
    }
}
