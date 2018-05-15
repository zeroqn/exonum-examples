use exonum::crypto::{CryptoHash, Hash};
use exonum::blockchain::Transaction;
use exonum_testkit::{ApiKind, TestKit, TestKitApi};

use SERVICE_NAME;
use schema::{ProposalList, Schema as BallotSchema};
use api::{BallotHashInfo, BallotResponse, VoteRequest, VoteResponse, VotesInfo};
use tests::common::*;
use tests::tx_logic::BallotTestKit;

fn forge_ballot_hash_info(testkit: &TestKit, proposals: ProposalList) -> BallotHashInfo {
    let snapshot = testkit.snapshot();
    let schema = BallotSchema::new(snapshot);

    let proposals_hash = proposals.hash();
    let ballot_data = schema
        .ballot_data_by_proposals_hash()
        .get(&proposals_hash)
        .expect("Data for ballot is absent");
    let votes = Some(schema.votes(&proposals_hash));
    let hash = ballot_data.hash();

    BallotHashInfo {
        ballot: Some(ballot_data),
        hash: Some(hash),
        proposals: Some(proposals),
        proposals_hash,
        votes,
    }
}

trait BallotApiTest {
    fn range_ballots(&self, offset: u64, limit: usize) -> Vec<BallotHashInfo>;

    fn ballot_by_hash(&self, proposals_hash: &Hash) -> BallotHashInfo;

    fn votes_for_ballot(&self, proposals_hash: &Hash) -> VotesInfo;

    fn post_ballot(&self, proposals: &ProposalList) -> BallotResponse;

    fn post_vote(&self, proposals_hash: &Hash, vote_req: &VoteRequest) -> VoteResponse;
}

impl BallotApiTest for TestKitApi {
    fn range_ballots(&self, offset: u64, limit: usize) -> Vec<BallotHashInfo> {
        self.get(
            ApiKind::Service(SERVICE_NAME),
            &format!("/v1/ballots?limit={}&offset={}", limit, offset),
        )
    }

    fn ballot_by_hash(&self, proposals_hash: &Hash) -> BallotHashInfo {
        self.get(
            ApiKind::Service(SERVICE_NAME),
            &format!("/v1/ballots/{}", proposals_hash),
        )
    }

    fn votes_for_ballot(&self, proposals_hash: &Hash) -> VotesInfo {
        self.get(
            ApiKind::Service(SERVICE_NAME),
            &format!("/v1/ballots/{}/votes", proposals_hash),
        )
    }

    fn post_ballot(&self, proposals: &ProposalList) -> BallotResponse {
        self.post_private(ApiKind::Service(SERVICE_NAME), "/v1/ballots", proposals)
    }

    fn post_vote(&self, proposals_hash: &Hash, vote_req: &VoteRequest) -> VoteResponse {
        self.post_private(
            ApiKind::Service(SERVICE_NAME),
            &format!("/v1/ballots/{}/postvote", proposals_hash),
            &vote_req,
        )
    }
}

#[test]
fn test_ranges_ballots() {
    let mut testkit: TestKit = TestKit::ballot_default();
    let proposals_list = vec![
        r#"{"id": 1, "deadline": 100, "proposals": [{"id": 1, "subject": "triss", "description": "magic"}]}"#,
        r#"{"id": 2, "deadline": 100, "proposals": [{"id": 1, "subject": "ciri", "description": "queen"}]}"#,
        r#"{"id": 3, "deadline": 100, "proposals": [{"id": 1, "subject": "yennefer", "description": "magic"}]}"#,
    ];
    let proposals_list = proposals_list
        .iter()
        .map(|json| ProposalList::try_deserialize(json.as_bytes()).unwrap())
        .collect::<Vec<ProposalList>>();
    let tx_ballot_list = proposals_list
        .iter()
        .map(|proposals| new_tx_ballot(&testkit.network().validators()[0], proposals.clone()))
        .map(|tx| Box::new(tx) as Box<Transaction>)
        .collect::<Vec<_>>();
    testkit.create_block_with_transactions(tx_ballot_list);

    let ballots_info = proposals_list
        .into_iter()
        .map(|proposals| forge_ballot_hash_info(&testkit, proposals))
        .collect::<Vec<BallotHashInfo>>();

    let resp = testkit.api().range_ballots(0, 10);
    assert_eq!(ballots_info, resp);

    let resp = testkit.api().range_ballots(1, 10);
    assert_eq!(ballots_info[1..].to_vec(), resp);

    let resp = testkit.api().range_ballots(0, 1);
    assert_eq!(vec![ballots_info[0].clone()], resp);
}

#[test]
fn test_ballot_by_hash_with_proofs() {
    let mut testkit: TestKit = TestKit::ballot_default();
    let (_, proposals) = create_test_ballot!(testkit);
    let proposals_hash = proposals.hash();
    let info = forge_ballot_hash_info(&testkit, proposals);

    let resp = testkit.api().ballot_by_hash(&proposals_hash);
    assert_eq!(info, resp);
}

#[test]
fn test_votes_for_ballot() {
    use exonum::blockchain::Schema;

    let mut testkit: TestKit = TestKit::ballot_default();
    let api = testkit.api();

    let (_, proposals) = new_proposals_data();
    let proposals_hash = proposals.hash();
    let tx_ballot = new_tx_ballot(&testkit.network().validators()[0], proposals);
    assert_eq!(None, api.votes_for_ballot(&proposals_hash));
    testkit.create_block_with_transaction(tx_ballot);
    assert_eq!(
        Some(vec![None; testkit.network().validators().len()]),
        api.votes_for_ballot(&proposals_hash)
    );

    let vote_req = VoteRequest {
        proposal_id: 1,
        proposal_subject: "triss".to_string(),
    };
    let tx_votes = testkit
        .network()
        .validators()
        .iter()
        .map(|validator| new_tx_vote(validator, &proposals_hash, &vote_req))
        .map(|tx| Box::new(tx) as Box<Transaction>)
        .collect::<Vec<_>>();
    testkit.create_block_with_transactions(tx_votes);
    let resp = api.votes_for_ballot(&proposals_hash)
        .expect("Votes for ballot are absent");
    for entry in resp.into_iter().take(testkit.network().validators().len()) {
        let tx = entry.expect("Vote for ballot is absent");

        assert!(
            Schema::new(&testkit.snapshot())
                .transactions()
                .contains(&tx.hash()),
            "Transaction is absent in blockchain: {:?}",
            tx
        );
    }
}

#[test]
fn test_post_ballot() {
    let mut testkit: TestKit = TestKit::ballot_default();
    let api = testkit.api();
    let (_, proposals) = new_proposals_data();

    let resp = api.post_ballot(&proposals);
    testkit.poll_events();

    let tx = new_tx_ballot(&testkit.network().validators()[0], proposals);
    assert_eq!(tx.hash(), resp.tx_hash);
    assert!(testkit.is_tx_in_pool(&resp.tx_hash));
}

#[test]
fn test_post_vote() {
    let mut testkit: TestKit = TestKit::ballot_default();
    let api = testkit.api();
    let (_, proposals) = create_test_ballot!(testkit);

    let vote_req = VoteRequest {
        proposal_id: 1,
        proposal_subject: "triss".to_string(),
    };
    let resp = api.post_vote(&proposals.hash(), &vote_req);
    testkit.poll_events();

    let tx = new_tx_vote(
        &testkit.network().validators()[0],
        &proposals.hash(),
        &vote_req,
    );
    assert_eq!(tx.hash(), resp.tx_hash);
    assert!(testkit.is_tx_in_pool(&resp.tx_hash));
}
