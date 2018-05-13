use exonum::blockchain::ExecutionError;

use serde_json::Error as JsonError;

#[derive(Debug)]
#[repr(u8)]
pub enum ErrorCode {
    BallotNoneExists = 0,
    BallotAlreadyPosted = 1,
    InvalidProposals = 2,
    PostDuplicateProposalId = 3,
    UnknownSender = 4,
    VotedProposalNoneExists = 5,
    AlreadyVoted = 6,
    InternalError = 255,
}

#[derive(Debug, Fail)]
pub(crate) enum Error {
    #[fail(display = "Ballot doesn't exist")]
    BallotNoneExists,

    #[fail(display = "Ballot already Posted")]
    BallotAlreadyPosted,

    #[fail(display = "Invalid proposals json: {}", _0)]
    InvalidProposals(#[cause] JsonError),

    #[fail(display = "Posted proposals contain duplicate id")]
    PostDuplicateProposalId,

    #[fail(display = "Not authored by a validator")]
    UnknownSender,

    #[fail(display = "Voted proposal doesn't exist")]
    VotedProposalNoneExists,

    #[fail(display = "Already Voted")]
    AlreadyVoted,

    #[fail(display = "Internal Error {}", _0)]
    InternalError(String),
}

impl Error {
    fn code(&self) -> ErrorCode {
        use self::Error::*;

        match *self {
            BallotNoneExists => ErrorCode::BallotNoneExists,
            BallotAlreadyPosted => ErrorCode::BallotAlreadyPosted,
            InvalidProposals(_) => ErrorCode::InvalidProposals,
            PostDuplicateProposalId => ErrorCode::PostDuplicateProposalId,
            UnknownSender => ErrorCode::UnknownSender,
            VotedProposalNoneExists => ErrorCode::VotedProposalNoneExists,
            AlreadyVoted => ErrorCode::AlreadyVoted,
            InternalError(_) => ErrorCode::InternalError,
        }
    }
}

impl From<Error> for ExecutionError {
    fn from(value: Error) -> ExecutionError {
        ExecutionError::with_description(value.code() as u8, value.to_string())
    }
}
