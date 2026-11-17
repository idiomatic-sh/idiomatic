//! The `ReadSets` value's cases.

use super::value::ReadSets;
use crate::domain::trust::types::relation_read::value::RelationRead;
use crate::domain::trust::types::{ReadSetHash, SetSharing, StoredSets};
use ryo_relations::domain::relations::types::Relation;

/// A read of all of `relation`.
fn read(relation: Relation) -> RelationRead {
    RelationRead::whole(relation)
}

#[test]
fn equal_reads_are_stored_once() {
    let mut sets = ReadSets::default();
    let first = sets.shared(vec![read(Relation::Defines)]);
    let second = sets.shared(vec![read(Relation::Defines)]);
    assert_eq!(
        (first.sharing(&second), sets.distinct()),
        (SetSharing::Shared, StoredSets(1))
    );
}

#[test]
fn reads_equal_but_hashed_together_stay_distinct() {
    let mut sets = ReadSets::default();
    let first = sets.stored_by(vec![read(Relation::Defines)], ReadSetHash(7));
    let second = sets.stored_by(vec![read(Relation::Function)], ReadSetHash(7));
    assert_eq!(
        (first.sharing(&second), sets.distinct(), &*second),
        (
            SetSharing::Copied,
            StoredSets(2),
            &[read(Relation::Function)][..]
        )
    );
}
