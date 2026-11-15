//! Literal-mask setup work, exact vocabulary identity and generation isolation.

use crate::domain::kind_sets::service::prepare_vocabulary;
use crate::domain::operator::types::OperatorContext;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::table_scratch::types::OperatorScratch;
use ryo_operators::domain::tree_cells::types::{TreeForm, TreeRows};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};

/// Build a small real vocabulary; the payload is irrelevant to prepared kind predicates.
fn prepared_tree(names: &[&str]) -> (Elements, Symbols) {
    let mut elements = Elements::default();
    let mut symbols = Symbols::default();
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"a"));
    enter_node(&mut builder, KindBits(0), Offset(0));
    token(&mut builder, KindBits(1), Offset(0), Offset(1));
    leave_node(&mut builder, Offset(1));
    for (index, name) in names.iter().enumerate() {
        name_kind(
            &mut completion(&mut elements, &mut symbols, ByteSlice(b"a"), FileId(0)),
            KindBits(u16::try_from(index).unwrap()),
            NameText(name),
        );
    }
    (elements, symbols)
}

/// A real collision in the old additive KindSum cannot reuse a differently numbered vocabulary.
#[test]
fn equal_kind_sums_do_not_identify_prepared_vocabularies() {
    use ryo_language::domain::elements::service::kind_vocabulary;
    use ryo_language::domain::symbols::service::hash_of;
    let names = ["word", "file", "node"];
    let (left, right) = names
        .iter()
        .enumerate()
        .find_map(|(index, first)| {
            names
                .iter()
                .skip(index + 1)
                .find(|second| {
                    hash_of(ByteSlice(first.as_bytes())).0 & 1
                        == hash_of(ByteSlice(second.as_bytes())).0 & 1
                })
                .map(|second| (*first, *second))
        })
        .unwrap();
    let (first, first_symbols) = prepared_tree(&[left, right]);
    let (second, second_symbols) = prepared_tree(&[right, left]);
    assert_eq!(kind_vocabulary(&first), kind_vocabulary(&second));
    let mut previous = None;
    let a = prepare_vocabulary(&mut previous, &first, &first_symbols, SourceView(b"a")).unwrap();
    let b = prepare_vocabulary(&mut previous, &second, &second_symbols, SourceView(b"a")).unwrap();
    assert!(!std::sync::Arc::ptr_eq(&a, &b));
    assert_ne!(a.names, b.names);
    assert_eq!(
        a.names.first().unwrap().as_ref().map(|name| &*name.0),
        Some(left)
    );
    assert_eq!(
        b.names.first().unwrap().as_ref().map(|name| &*name.0),
        Some(right)
    );
}

/// Prepared metadata can cross worker ownership boundaries without retaining local source borrows.
#[test]
fn prepared_vocabulary_is_send_and_sync() {
    fn requires_send_sync<T: Send + Sync>() {}
    requires_send_sync::<crate::domain::kind_sets::types::vocabulary::value::PreparedVocabulary>();
}
