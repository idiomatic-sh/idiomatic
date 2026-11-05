//! The public publication boundary runs its positive and refusal cases.
use super::{fixtures::check, use_case::PublicationUseCaseImpl};
use ryo_values::cases::Word;
#[test]
fn portable_publication_contract() {
    check(&PublicationUseCaseImpl, Word::Succeeds);
    check(&PublicationUseCaseImpl, Word::Refuses);
}
