extern crate ballot;
extern crate exonum;
#[macro_use]
extern crate exonum_testkit;
extern crate rand;

use exonum::{crypto::{self, PublicKey, SecretKey}, storage::{ListIndex, Snapshot}};
use exonum_testkit::{TestKit, TestKitBuilder};

use ballot::{constants::{INIT_WEIGHT, MAX_PROPOSALS},
             models::{Chairperson, NewProposal, Voter, Voting}, schema::BallotSchema,
             service::BallotService,
             transactions::{TxANewChairperson, TxASetVoterActiveState, TxCreateVoter,
                            TxNewVoting, TxVoteProposal}};
use constants::*;

#[macro_use]
mod constants;

macro_rules! assert_votings {
    ($testkit:expr, $assert:expr) => {{
        let snapshot = $testkit.snapshot();
        let ballot = BallotSchema::new(&snapshot);
        let votings = ballot.votings();

        $assert(votings);
    }};
}

#[test]
fn test_create_voter() {
    let mut testkit = init_testkit();
    let (tx, pubkey, _) = create_voter_tx(ALICE_NAME);
    testkit.create_block_with_transaction(tx);

    let voter = get_voter(&testkit, &pubkey);
    assert_eq!(voter.pub_key(), &pubkey);
    assert_eq!(voter.name(), ALICE_NAME);
    assert_eq!(voter.weight(), INIT_WEIGHT);
    assert_eq!(voter.is_active(), true);
}

#[test]
fn test_first_voter_is_chairperson() {
    let mut testkit = init_testkit();
    let (tx, pubkey, _) = create_voter_tx(ALICE_NAME);
    testkit.create_block_with_transaction(tx);

    let chairperson = get_chairperson(&testkit);
    assert_eq!(chairperson.pub_key(), &pubkey);
    assert_eq!(chairperson.name(), ALICE_NAME);
}

#[test]
fn test_change_chairperson() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let (tx_create_bob, bob_pubkey, _) = create_voter_tx(BOB_NAME);
    let tx_change_chairperson_to_bob =
        TxANewChairperson::new(&alice_pubkey, &bob_pubkey, &alice_key);

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_create_bob,
        tx_change_chairperson_to_bob
    ]);

    let chairperson = get_chairperson(&testkit);
    assert_eq!(chairperson.pub_key(), &bob_pubkey);
    assert_eq!(chairperson.name(), BOB_NAME);
}

#[test]
fn test_change_chairperson_without_permission() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, _alice_key) = create_voter_tx(ALICE_NAME);
    let (tx_create_bob, bob_pubkey, bob_key) = create_voter_tx(BOB_NAME);
    let tx_change_chairperson_to_bob = TxANewChairperson::new(&bob_pubkey, &bob_pubkey, &bob_key);

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_create_bob,
        tx_change_chairperson_to_bob
    ]);

    let chairperson = get_chairperson(&testkit);
    assert_eq!(chairperson.pub_key(), &alice_pubkey);
    assert_eq!(chairperson.name(), ALICE_NAME);
}

#[test]
fn test_change_chairperson_to_inactive_voter() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let (tx_create_bob, bob_pubkey, _bob_key) = create_voter_tx(BOB_NAME);
    let tx_deactive_bob =
        TxASetVoterActiveState::new(&alice_pubkey, &bob_pubkey, false, &alice_key);
    let tx_change_chairperson_to_bob =
        TxANewChairperson::new(&alice_pubkey, &bob_pubkey, &alice_key);

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_create_bob,
        tx_deactive_bob,
        tx_change_chairperson_to_bob
    ]);

    let bob = get_voter(&testkit, &bob_pubkey);
    let chairperson = get_chairperson(&testkit);
    assert_eq!(bob.is_active(), false);
    assert_eq!(chairperson.pub_key(), &alice_pubkey);
    assert_eq!(chairperson.name(), ALICE_NAME);
}

#[test]
fn test_set_voter_active_state() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let (tx_create_bob, bob_pubkey, _bob_key) = create_voter_tx(BOB_NAME);
    let tx_deactive_bob =
        TxASetVoterActiveState::new(&alice_pubkey, &bob_pubkey, false, &alice_key);

    testkit.create_block_with_transactions(txvec![tx_create_alice, tx_create_bob, tx_deactive_bob]);

    let bob = get_voter(&testkit, &bob_pubkey);
    assert_eq!(bob.is_active(), false);

    let tx_active_bob = TxASetVoterActiveState::new(&alice_pubkey, &bob_pubkey, true, &alice_key);
    testkit.create_block_with_transaction(tx_active_bob);

    let bob = get_voter(&testkit, &bob_pubkey);
    assert_eq!(bob.is_active(), true);
}

#[test]
fn test_chairperson_set_self_active_state() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let tx_deactive_chairperson_alice =
        TxASetVoterActiveState::new(&alice_pubkey, &alice_pubkey, false, &alice_key);

    testkit.create_block_with_transactions(txvec![tx_create_alice, tx_deactive_chairperson_alice]);

    let alice = get_voter(&testkit, &alice_pubkey);
    assert_eq!(alice.is_active(), true);
}

#[test]
fn test_set_voter_without_chairperson_permission() {
    let mut testkit = init_testkit();
    let (tx_create_alice, _alice_pubkey, _alice_key) = create_voter_tx(ALICE_NAME);
    let (tx_create_bob, bob_pubkey, bob_key) = create_voter_tx(BOB_NAME);
    let (tx_create_triss, triss_pubkey, _triss_key) = create_voter_tx(TRISS);

    let tx_deactive_triss_by_bob =
        TxASetVoterActiveState::new(&bob_pubkey, &triss_pubkey, false, &bob_key);

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_create_bob,
        tx_create_triss,
        tx_deactive_triss_by_bob
    ]);

    let triss = get_voter(&testkit, &triss_pubkey);
    assert_eq!(triss.is_active(), true);
}

#[test]
fn test_new_voting() {
    let mut testkit = init_testkit();

    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let test_subjects: Vec<&str> = get_subjects!();
    let tx_new_voting = new_voting_tx(&test_subjects, &alice_pubkey, &alice_key);

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_new_voting.clone(),
        tx_new_voting.clone()
    ]);

    assert_votings!(testkit, |votings: ListIndex<&Snapshot, Voting>| {
        assert_eq!(votings.len(), 2);
        let voting = votings.last().unwrap();

        assert_eq!(voting.id(), 1);
        assert!(voting.voted_voters().is_empty());
        for (idx, proposal) in voting.proposals().iter().enumerate() {
            assert_eq!(proposal.subject(), test_subjects[idx]);
            assert_eq!(proposal.vote_count(), 0);
        }
    });
}

#[test]
fn test_new_voting_with_excess_max_proposals() {
    let mut testkit = init_testkit();

    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);

    let test_proposals: Vec<String> = (1..MAX_PROPOSALS + 2).map(|n| n.to_string()).collect();
    let test_proposals: Vec<&str> = test_proposals.iter().map(|n| &n[..]).collect();
    let tx_new_voting = new_voting_tx(&test_proposals, &alice_pubkey, &alice_key);

    testkit.create_block_with_transactions(txvec![tx_create_alice, tx_new_voting]);

    assert_votings!(testkit, |votings: ListIndex<&Snapshot, Voting>| {
        assert!(votings.is_empty());
    });
}

#[test]
fn test_new_voting_without_voter_permission() {
    let mut testkit = init_testkit();
    let (alice_pubkey, alice_key) = crypto::gen_keypair();

    let tx = new_voting_tx(&(get_subjects!()), &alice_pubkey, &alice_key);
    testkit.create_block_with_transaction(tx);

    assert_votings!(testkit, |votings: ListIndex<&Snapshot, Voting>| {
        assert!(votings.is_empty());
    });
}

#[test]
fn test_vote_proposal() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let (tx_create_bob, bob_pubkey, bob_key) = create_voter_tx(BOB_NAME);
    let tx_new_voting = new_voting_tx(&(get_subjects!()), &alice_pubkey, &alice_key);
    let expect_voting_id = 0;
    let expect_proposal_id = 1;
    let tx_bob_vote_second_proposal =
        TxVoteProposal::new(&bob_pubkey, expect_voting_id, expect_proposal_id, &bob_key);

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_create_bob,
        tx_new_voting,
        tx_bob_vote_second_proposal
    ]);

    let bob = get_voter(&testkit, &bob_pubkey);

    assert!(has_voted(&testkit, expect_voting_id, &bob_pubkey));
    assert_votings!(testkit, |votings: ListIndex<&Snapshot, Voting>| {
        let voting = votings.get(expect_voting_id).unwrap();
        let proposal = voting.get_proposal(expect_proposal_id as usize).unwrap();
        assert!(voting.has_voted(&bob_pubkey));
        assert_eq!(proposal.vote_count(), bob.weight());
    });
}

#[test]
fn test_vote_proposal_without_vote_permission() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let (_, bob_pubkey, bob_key) = create_voter_tx(BOB_NAME);
    let tx_new_voting = new_voting_tx(&(get_subjects!()), &alice_pubkey, &alice_key);
    let expect_voting_id = 0;
    let expect_proposal_id = 1;
    let tx_bob_vote_second_proposal =
        TxVoteProposal::new(&bob_pubkey, expect_voting_id, expect_proposal_id, &bob_key);

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_new_voting,
        tx_bob_vote_second_proposal
    ]);

    assert!(!has_voted(&testkit, expect_voting_id, &bob_pubkey));
    assert_votings!(testkit, |votings: ListIndex<&Snapshot, Voting>| {
        let voting = votings.get(expect_voting_id).unwrap();
        let proposal = voting.get_proposal(expect_proposal_id as usize).unwrap();
        assert_eq!(proposal.vote_count(), 0);
        assert!(!voting.has_voted(&bob_pubkey));
    });
}

#[test]
fn test_vote_proposal_twice() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let tx_new_voting = new_voting_tx(&(get_subjects!()), &alice_pubkey, &alice_key);
    let expect_voting_id = 0;
    let expect_proposal_id = 0;
    let tx_alice_vote_first_proposal = TxVoteProposal::new(
        &alice_pubkey,
        expect_voting_id,
        expect_proposal_id,
        &alice_key,
    );

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_new_voting,
        tx_alice_vote_first_proposal.clone(),
        tx_alice_vote_first_proposal.clone(),
    ]);

    let alice = get_voter(&testkit, &alice_pubkey);

    assert!(has_voted(&testkit, expect_voting_id, &alice_pubkey));
    assert_votings!(testkit, |votings: ListIndex<&Snapshot, Voting>| {
        let voting = votings.get(expect_voting_id).unwrap();
        let proposal = voting.get_proposal(expect_proposal_id as usize).unwrap();
        assert_eq!(proposal.vote_count(), alice.weight());
        assert!(voting.has_voted(&alice_pubkey));
    });
}

#[test]
fn test_vote_none_exist_proposal() {
    let mut testkit = init_testkit();
    let (tx_create_alice, alice_pubkey, alice_key) = create_voter_tx(ALICE_NAME);
    let tx_new_voting = new_voting_tx(&(get_subjects!()), &alice_pubkey, &alice_key);
    let expect_voting_id = 0;
    let expect_proposal_id = get_subjects!().len() + 1;
    let tx_alice_vote_none_exist_proposal = TxVoteProposal::new(
        &alice_pubkey,
        expect_voting_id,
        expect_proposal_id as u16,
        &alice_key,
    );

    testkit.create_block_with_transactions(txvec![
        tx_create_alice,
        tx_new_voting,
        tx_alice_vote_none_exist_proposal
    ]);

    assert!(!has_voted(&testkit, expect_voting_id, &alice_pubkey));
    assert_votings!(testkit, |votings: ListIndex<&Snapshot, Voting>| {
        let voting = votings.get(expect_voting_id).unwrap();
        assert!(!voting.has_voted(&alice_pubkey));
    });
}

fn init_testkit() -> TestKit {
    TestKitBuilder::validator()
        .with_service(BallotService)
        .create()
}

fn create_voter_tx(name: &str) -> (TxCreateVoter, PublicKey, SecretKey) {
    let (pubkey, key) = crypto::gen_keypair();
    (TxCreateVoter::new(&pubkey, name, &key), pubkey, key)
}

fn new_voting_tx(subjects: &Vec<&str>, pubkey: &PublicKey, key: &SecretKey) -> TxNewVoting {
    let mut proposals: Vec<NewProposal> = vec![];
    for subject in subjects {
        proposals.push(NewProposal::new(subject));
    }

    TxNewVoting::new(pubkey, proposals, key)
}

fn get_schema(testkit: &TestKit) -> BallotSchema<Box<Snapshot>> {
    BallotSchema::new(testkit.snapshot())
}

fn try_get_voter(testkit: &TestKit, pubkey: &PublicKey) -> Option<Voter> {
    get_schema(testkit).voter(pubkey)
}

fn get_voter(testkit: &TestKit, pubkey: &PublicKey) -> Voter {
    try_get_voter(testkit, pubkey).expect("No voter persisted")
}

fn get_chairperson(testkit: &TestKit) -> Chairperson {
    get_schema(testkit)
        .chairperson()
        .expect("No chairperson persisted")
}

fn has_voted(testkit: &TestKit, id: u64, pub_key: &PublicKey) -> bool {
    get_schema(testkit).has_voted(id, pub_key)
}
