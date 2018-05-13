extern crate ballot;
extern crate exonum;
#[macro_use]
extern crate exonum_testkit;
extern crate rand;

use exonum::crypto::{self, hash, CryptoHash, Hash};
use exonum::storage::StorageValue;
use exonum_testkit::{TestKit, TestKitBuilder, TestNode};

use ballot::BallotService;
use ballot::schema::{ProposalList, Schema as BallotSchema};
use ballot::transactions::{Ballot, Vote};
use ballot::error::ErrorCode;

macro_rules! create_test_ballot {
    ($testkit: expr) => {{
        let (_, proposals) = new_proposals_data();
        let tx_ballot = new_tx_ballot(&$testkit.network().validators()[1], proposals.clone());
        $testkit.create_block_with_transaction(tx_ballot.clone());
        assert_eq!(tx_ballot, $testkit.find_ballot(&proposals.hash()).unwrap());

        (tx_ballot, proposals)
    }}
}

macro_rules! assert_error_code {
    ($snapshot: expr, $tx_hash: expr, $err_code: expr) => {{
        use exonum::blockchain::{Schema, TransactionErrorType};

        let tx_error = Schema::new($snapshot)
            .transaction_results()
            .get($tx_hash)
            .unwrap()
            .unwrap_err();

        assert_eq!(
            tx_error.error_type(),
            TransactionErrorType::Code($err_code as u8)
            );
    }}
}

struct VoteRequest<'a> {
    proposals_hash: &'a Hash,
    proposal_id: u64,
    proposal_subject: &'a str,
}

fn new_tx_ballot(node: &TestNode, proposals: ProposalList) -> Ballot {
    let keypair = node.service_keypair();
    Ballot::new(
        keypair.0,
        ::std::str::from_utf8(proposals.into_bytes().as_slice()).unwrap(),
        keypair.1,
    )
}

fn new_tx_vote(node: &TestNode, vote_req: VoteRequest) -> Vote {
    let keypair = node.service_keypair();
    Vote::new(
        keypair.0,
        vote_req.proposals_hash,
        vote_req.proposal_id,
        vote_req.proposal_subject,
        keypair.1,
    )
}

fn new_proposals_data() -> (String, ProposalList) {
    let proposals_str = r#"{"id": 1,"proposals":[
                    {"id": 1, "subject": "triss", "description": "magic"}
                  , {"id": 2, "subject": "ciri", "description": "queen"}
                  , {"id": 3, "subject": "yennefer", "description": "magic"}
                  ]}"#;
    let proposals = ProposalList::try_deserialize(proposals_str.as_bytes()).unwrap();
    (proposals_str.to_string(), proposals)
}

trait BallotTestKit {
    fn ballot_default() -> Self;

    fn find_ballot(&self, proposals_hash: &Hash) -> Option<Ballot>;

    fn votes(&self, proposals_hash: &Hash) -> Vec<Option<Vote>>;
}

impl BallotTestKit for TestKit {
    fn ballot_default() -> Self {
        TestKitBuilder::validator()
            .with_validators(4)
            .with_service(BallotService)
            .create()
    }

    fn find_ballot(&self, proposals_hash: &Hash) -> Option<Ballot> {
        let snapshot = self.snapshot();
        let schema = BallotSchema::new(&snapshot);
        schema.ballot(&proposals_hash)
    }

    fn votes(&self, proposals_hash: &Hash) -> Vec<Option<Vote>> {
        let snapshot = self.snapshot();
        let schema = BallotSchema::new(&snapshot);
        schema.votes(proposals_hash)
    }
}

#[test]
fn test_post_ballot() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let (_, proposals) = new_proposals_data();
    let tx_ballot = new_tx_ballot(&testkit.network().validators()[1], proposals.clone());
    testkit.create_block_with_transaction(tx_ballot.clone());

    assert_eq!(tx_ballot, testkit.find_ballot(&proposals.hash()).unwrap());
}

#[test]
fn test_post_ballot_with_unknown_sender() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let (ref proposals_str, _) = new_proposals_data();
    let tx_ballot = {
        let keypair = crypto::gen_keypair();
        Ballot::new(&keypair.0, proposals_str, &keypair.1)
    };
    testkit.create_block_with_transaction(tx_ballot.clone());

    assert_error_code!(
        &testkit.snapshot(),
        &tx_ballot.hash(),
        ErrorCode::UnknownSender
    );
}

#[test]
fn test_post_ballot_with_invalid_proposals() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let invalid_proposals_str = r#"{"proposals": ["xxx": 222]}"#;
    let tx_ballot = {
        let keypair = testkit.network().validators()[1].service_keypair();
        Ballot::new(&keypair.0, invalid_proposals_str, &keypair.1)
    };
    testkit.create_block_with_transaction(tx_ballot.clone());

    assert_eq!(
        None,
        testkit.find_ballot(&hash(invalid_proposals_str.as_bytes()))
    );
    assert_error_code!(
        &testkit.snapshot(),
        &tx_ballot.hash(),
        ErrorCode::InvalidProposals
    );
}

#[test]
fn test_post_duplicate_ballot() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let (_, proposals) = new_proposals_data();
    let tx_ballot = new_tx_ballot(&testkit.network().validators()[1], proposals.clone());
    let tx_dup_ballot = new_tx_ballot(&testkit.network().validators()[2], proposals.clone());
    testkit.create_block_with_transactions(txvec![tx_ballot, tx_dup_ballot.clone()]);

    assert_error_code!(
        &testkit.snapshot(),
        &tx_dup_ballot.hash(),
        ErrorCode::BallotAlreadyPosted
    );
}

#[test]
fn test_post_ballot_with_duplicate_proposal_id() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let proposals_str =
        r#"{"id": 1,"proposals": [{"id": 1, "subject": "triss", "description": "magic"}
                                , {"id": 1, "subject": "ciri", "description": "hunter"}
                                 ]}"#;
    let proposals = ProposalList::try_deserialize(proposals_str.as_bytes()).unwrap();

    let tx_ballot = new_tx_ballot(&testkit.network().validators()[1], proposals.clone());
    testkit.create_block_with_transaction(tx_ballot.clone());

    assert_error_code!(
        &testkit.snapshot(),
        &tx_ballot.hash(),
        ErrorCode::PostDuplicateProposalId
    );
}

#[test]
fn test_post_vote() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let (_, proposals) = create_test_ballot!(testkit);
    let vote_req = VoteRequest {
        proposals_hash: &proposals.hash(),
        proposal_id: 1,
        proposal_subject: "triss",
    };
    let tx_vote = new_tx_vote(&testkit.network().validators()[1], vote_req);

    let votes = testkit.votes(&proposals.hash());
    assert!(!votes.contains(&Some(tx_vote.clone())));

    testkit.create_block_with_transaction(tx_vote.clone());
    let votes = testkit.votes(&proposals.hash());
    assert!(votes.contains(&Some(tx_vote)));
}

#[test]
fn test_post_vote_from_unknown_sender() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let (_, proposals) = create_test_ballot!(testkit);
    let tx_vote = {
        let keypair = crypto::gen_keypair();
        Vote::new(&keypair.0, &proposals.hash(), 1, "triss", &keypair.1)
    };
    testkit.create_block_with_transaction(tx_vote.clone());

    let votes = testkit.votes(&proposals.hash());
    assert!(!votes.contains(&Some(tx_vote.clone())));
    assert_error_code!(
        &testkit.snapshot(),
        &tx_vote.hash(),
        ErrorCode::UnknownSender
    );
}

#[test]
fn test_post_vote_for_none_exist_ballot() {
    let mut testkit: TestKit = TestKit::ballot_default();

    create_test_ballot!(testkit);
    let vote_req = VoteRequest {
        proposals_hash: &hash("wrong".as_bytes()),
        proposal_id: 1,
        proposal_subject: "triss",
    };
    let tx_vote = new_tx_vote(&testkit.network().validators()[1], vote_req);
    testkit.create_block_with_transaction(tx_vote.clone());

    assert_error_code!(
        &testkit.snapshot(),
        &tx_vote.hash(),
        ErrorCode::BallotNoneExists
    );
}

#[test]
fn test_post_vote_twice() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let (_, proposals) = create_test_ballot!(testkit);
    let vote_req = VoteRequest {
        proposals_hash: &proposals.hash(),
        proposal_id: 1,
        proposal_subject: "triss",
    };
    let tx_vote = new_tx_vote(&testkit.network().validators()[1], vote_req);
    let vote_req = VoteRequest {
        proposals_hash: &proposals.hash(),
        proposal_id: 2,
        proposal_subject: "ciri",
    };
    let tx_illegal_vote = new_tx_vote(&testkit.network().validators()[1], vote_req);
    testkit.create_block_with_transactions(txvec![tx_vote.clone(), tx_illegal_vote.clone()]);

    let votes = testkit.votes(&proposals.hash());
    assert!(votes.contains(&Some(tx_vote)));
    assert_error_code!(
        &testkit.snapshot(),
        &tx_illegal_vote.hash(),
        ErrorCode::AlreadyVoted
    );
}

#[test]
fn test_post_vote_for_none_exist_proposal() {
    let mut testkit: TestKit = TestKit::ballot_default();

    let (_, proposals) = create_test_ballot!(testkit);
    let vote_req = VoteRequest {
        proposals_hash: &proposals.hash(),
        proposal_id: 99,
        proposal_subject: "wrong",
    };
    let tx_vote = new_tx_vote(&testkit.network().validators()[1], vote_req);
    testkit.create_block_with_transaction(tx_vote.clone());

    assert_error_code!(
        &testkit.snapshot(),
        &tx_vote.hash(),
        ErrorCode::VotedProposalNoneExists
    );
}
