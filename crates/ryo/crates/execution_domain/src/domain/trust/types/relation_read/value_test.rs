//! The `RelationRead` value's cases.

use super::value::RelationRead;
use ryo_relations::domain::relations::types::Relation;

#[test]
fn a_whole_read_is_of_its_relation_narrowed_by_nothing() {
    let read = RelationRead::whole(Relation::Defines);
    assert_eq!((read.relation(), read.kinds.len()), (Relation::Defines, 0));
}
