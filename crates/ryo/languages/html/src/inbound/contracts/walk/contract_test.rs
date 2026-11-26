//! The `walk` contract's cases: the language's kinds and fields are named at the codes the table holds, and its linked provider hands the
//! engine the numbered visit. What the walk writes is held to the frozen rows by the `routes` target.

use super::contract::WalkPorts;
use crate::domain::file_view::types::HtmlWalk;
use crate::domain::syntax::service::{field_bits, field_name, fields, kind_bits, kind_name, kind_of, kinds};
use ryo::languages::{Fillers, Walk};

#[test]
fn the_kind_count_is_the_count_of_the_languages_kinds() {
    assert_eq!(<HtmlWalk<WalkPorts> as Walk>::KIND_COUNT.0, kinds().count());
}

#[test]
fn the_walk_names_each_kind_and_each_field_at_its_code() {
    let names = <HtmlWalk<WalkPorts> as Fillers>::vocabulary();
    for kind in kinds() {
        assert_eq!(kind_of(kind_bits(kind)), kind);
        assert_eq!(names.kinds.0.get(usize::from(kind_bits(kind).0)), Some(&kind_name(kind)));
    }
    for field in fields() {
        assert_eq!(names.fields.0.get(usize::from(field_bits(field).0)), Some(&field_name(field)));
    }
}

#[test]
fn the_linked_provider_hands_the_engine_its_numbered_visit() {
    let linked = crate::linked();
    let parse = linked.parse_engine();
    let relations = parse.file_relations();
    let _reader = relations.numbered().open();
}
