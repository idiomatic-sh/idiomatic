//! Unresolved mutable names defer. The permanent sentinel remains a stable symbol valued as empty text.
use super::super::entity_plan::value::QueryPlan;
use super::value::EntityFileColumns;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SYMBOL,
    service::kinds,
    types::{ColumnsView, FactColumn, Relation},
};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::files::types::SourceView;
type Input = Symbol;
type Value = bool;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_future_symbol_cannot_be_saved_with_an_empty_spelling"),
        word: Word::Succeeds,
        input: || Symbol(0),
        expect: Expect::Value(false),
    },
    Case {
        name: CaseName("the_permanent_missing_symbol_may_be_read_as_empty"),
        word: Word::Succeeds,
        input: || NO_SYMBOL,
        expect: Expect::Value(true),
    },
];
fn subject(raw: Input) -> Result<Value, Failure> {
    let symbols = Symbols::default();
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let graph = [0, u64::from(raw.0), u64::from(raw.0), 0, 0].map(|w| FactColumn(vec![w]));
    let child = std::array::from_fn::<_, 5, _>(|_| FactColumn(vec![]));
    let view = ColumnsView {
        kinds: kinds(Relation::SemanticSyntax),
        columns: &graph,
    };
    let edges = ColumnsView {
        kinds: kinds(Relation::SemanticChild),
        columns: &child,
    };
    Ok(EntityFileColumns::open(&QueryPlan::default(), view, edges, &texts).is_some())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject)
}
