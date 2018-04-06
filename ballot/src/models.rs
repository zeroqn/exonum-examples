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
