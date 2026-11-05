//! Success and refusal through the real public normalization implementation.
use super::{fixtures::check, use_case::PublicationUseCaseImpl};
use ryo_values::cases::Word;
#[test]
fn succeeds() {
    check(&PublicationUseCaseImpl, Word::Succeeds);
}
#[test]
fn refuses() {
    check(&PublicationUseCaseImpl, Word::Refuses);
}
