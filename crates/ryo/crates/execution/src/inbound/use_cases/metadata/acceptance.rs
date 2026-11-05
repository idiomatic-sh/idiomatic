//! The public surface also keeps the complete operator catalog and its descriptions paired.
use super::fixtures::contract;
use super::use_case::MetadataUseCaseImpl;

#[test]
fn descriptions_cover_the_listed_operators() {
    assert_eq!(contract::check(&MetadataUseCaseImpl), 25);
}
