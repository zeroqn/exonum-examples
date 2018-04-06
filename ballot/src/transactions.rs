use exonum::{blockchain::{ExecutionError, ExecutionResult, Transaction}, crypto::PublicKey,
             messages::Message, storage::Fork};

use constants::{INIT_WEIGHT, MAX_PROPOSALS, SERVICE_ID};
use models::{Chairperson, NewProposal, Proposal, Voter};
use schema::BallotSchema;

transactions!{
    pub BallotTransactions {
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
