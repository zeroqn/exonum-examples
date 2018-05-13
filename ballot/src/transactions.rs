use exonum::blockchain::{ExecutionResult, Schema as CoreSchema, Transaction};
use exonum::crypto::{CryptoHash, Hash, PublicKey};
use exonum::messages::Message;
use exonum::storage::{Fork, Snapshot};

use SERVICE_ID;
use schema::{BallotData, MaybeVote, ProposalList, Schema};
use error::Error as ServiceError;

fn validator_id(snapshot: &Snapshot, key: &PublicKey) -> Option<usize> {
    let actual_config = CoreSchema::new(snapshot).actual_configuration();
    let keys = actual_config.validator_keys;
    keys.iter().position(|k| k.service_key == *key)
}

transactions!{
    pub Transactions {
        const SERVICE_ID = SERVICE_ID;

        struct Ballot {
            from: &PublicKey,
            /// proposals json string
            proposals: &str,
        }

        struct Vote {
            from: &PublicKey,
            proposals_hash: &Hash,
            proposal_id: u64,
            proposal_subject: &str,
        }
    }
}

impl Ballot {
    fn precheck(&self, snapshot: &Snapshot) -> Result<ProposalList, ServiceError> {
        use self::ServiceError::*;

        if validator_id(snapshot, self.from()).is_none() {
            Err(UnknownSender)?
        }

        let proposals: ProposalList = ProposalList::try_deserialize(self.proposals().as_bytes())
            .map_err(|e| InvalidProposals(e))?;

        if proposals.has_duplicate_id() {
            Err(PostDuplicateProposalId)?
        }

        if Schema::new(snapshot)
            .ballot_data_by_proposals_hash()
            .get(&proposals.hash())
            .is_some()
        {
            Err(BallotAlreadyPosted)?
        }

        Ok(proposals)
    }

    fn save(&self, view: &mut Fork, proposals: ProposalList) {
        let proposals_hash = proposals.hash();
        let num_validators = {
            let core_config = CoreSchema::new(view.as_ref()).actual_configuration();
            core_config.validator_keys.len()
        };

        let mut schema = Schema::new(view);

        let ballot_data = {
            let mut votes_table = schema.votes_by_proposals_hash_mut(&proposals_hash);
            debug_assert!(votes_table.is_empty());

            for _ in 0..num_validators {
                votes_table.push(MaybeVote::none());
            }

            BallotData::new(
                self.clone(),
                &votes_table.merkle_root(),
                num_validators as u64,
            )
        };

        schema
            .ballot_data_by_proposals_hash_mut()
            .put(&proposals_hash, ballot_data);
        schema.proposals_hash_by_ordinal_mut().push(proposals_hash);
    }
}

impl Transaction for Ballot {
    fn verify(&self) -> bool {
        self.verify_signature(self.from())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        let proposals = self.precheck(view.as_ref())?;
        self.save(view, proposals);
        Ok(())
    }
}

impl Vote {
    fn precheck(&self, snapshot: &Snapshot) -> Result<(BallotData, usize), ServiceError> {
        use self::ServiceError::*;

        let validator_id = validator_id(snapshot, self.from()).ok_or(UnknownSender)?;
        let schema = Schema::new(snapshot);

        let ballot_data = schema
            .ballot_data_by_proposals_hash()
            .get(self.proposals_hash())
            .ok_or(BallotNoneExists)?;

        let vote = schema
            .votes_by_proposals_hash(self.proposals_hash())
            .get(validator_id as u64);
        if let Some(vote) = vote {
            if vote.is_some() {
                Err(AlreadyVoted)?
            }
        } else {
            Err(InternalError(format!(
                "Vote position isn't reserve. Sender: {}",
                self.from()
            )))?;
        }

        let proposals = ProposalList::try_deserialize(
            ballot_data.tx_ballot().proposals().as_bytes(),
        ).map_err(|e| InvalidProposals(e))?;
        if !proposals.contains(self.proposal_id(), self.proposal_subject()) {
            Err(VotedProposalNoneExists)?
        }

        Ok((ballot_data, validator_id))
    }

    fn save(&self, view: &mut Fork, ballot_data: BallotData, validator_id: u64) {
        let mut schema = Schema::new(view);

        let ballot_data = {
            let mut votes_table = schema.votes_by_proposals_hash_mut(self.proposals_hash());
            votes_table.set(validator_id, MaybeVote::some(self.clone()));

            BallotData::new(
                ballot_data.tx_ballot(),
                &votes_table.merkle_root(),
                validator_id,
            )
        };

        schema
            .ballot_data_by_proposals_hash_mut()
            .put(self.proposals_hash(), ballot_data);
    }
}

impl Transaction for Vote {
    fn verify(&self) -> bool {
        self.verify_signature(self.from())
    }

    fn execute(&self, view: &mut Fork) -> ExecutionResult {
        let (ballot_data, validator_id) = self.precheck(view.as_ref())?;
        self.save(view, ballot_data, validator_id as u64);
        Ok(())
    }
}
