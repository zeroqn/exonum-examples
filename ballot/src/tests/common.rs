use exonum::crypto::Hash;
use exonum::storage::StorageValue;
use exonum_testkit::TestNode;

use schema::ProposalList;
use transactions::{Ballot, Vote};
use api::VoteRequest;

macro_rules! create_test_ballot {
    ($testkit: expr) => {{
        let (_, proposals) = new_proposals_data();
        let tx_ballot = new_tx_ballot(&$testkit.network().validators()[0], proposals.clone());
        $testkit.create_block_with_transaction(tx_ballot.clone());
        assert_eq!(tx_ballot, $testkit.find_ballot(&proposals.hash()).unwrap());

        (tx_ballot, proposals)
    }}
}

pub fn new_proposals_data() -> (String, ProposalList) {
    let proposals_str = r#"{"id": 1, "deadline": 30, "proposals":[
                    {"id": 1, "subject": "triss", "description": "magic"}
                  , {"id": 2, "subject": "ciri", "description": "queen"}
                  , {"id": 3, "subject": "yennefer", "description": "magic"}
                  ]}"#;
    let proposals = ProposalList::try_deserialize(proposals_str.as_bytes()).unwrap();
    (proposals_str.to_string(), proposals)
}

pub fn new_tx_ballot(node: &TestNode, proposals: ProposalList) -> Ballot {
    let keypair = node.service_keypair();
    Ballot::new(
        keypair.0,
        ::std::str::from_utf8(proposals.into_bytes().as_slice()).unwrap(),
        keypair.1,
    )
}

pub fn new_tx_vote(node: &TestNode, proposals_hash: &Hash, vote_req: &VoteRequest) -> Vote {
    let keypair = node.service_keypair();
    Vote::new(
        keypair.0,
        proposals_hash,
        vote_req.proposal_id,
        &vote_req.proposal_subject,
        keypair.1,
    )
}
