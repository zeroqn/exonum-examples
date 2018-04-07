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

encoding_struct!{
    struct Voting {
        id: u64,
        proposals: Vec<Proposal>,
        voted_voters: Vec<VoterPubKey>,
    }
}

impl Voting {
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

        Some(Voting::new(self.id(), proposals, voted_voters))
    }

    pub fn has_voted(&self, voter_pubkey: &PublicKey) -> bool {
        self.voted_voters()
            .contains(&VoterPubKey::new(voter_pubkey))
    }

    pub fn get_proposal(&self, proposal_id: usize) -> Option<Proposal> {
        self.proposals().get(proposal_id).map(|p| p.to_owned())
    }
}
