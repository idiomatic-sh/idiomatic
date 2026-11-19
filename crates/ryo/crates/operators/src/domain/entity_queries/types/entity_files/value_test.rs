//! The file's stage cache belongs to the instance of the prepared query that established it.
use super::super::{EntityOwnership, entity_plan::value::QueryPlan};
use super::value::FileQueries;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::files::types::SourceView;
use std::sync::Arc;
#[derive(Clone, Copy)]
enum Scenario {
    Same,
    Other,
}
type Input = Scenario;
type Value = EntityOwnership;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("clones_of_the_prepared_plan_keep_the_file_cache"),
        word: Word::Succeeds,
        input: || Scenario::Same,
        expect: Expect::Value(EntityOwnership::Current),
    },
    Case {
        name: CaseName("a_different_prepared_plan_cannot_take_this_file_cache"),
        word: Word::Succeeds,
        input: || Scenario::Other,
        expect: Expect::Value(EntityOwnership::Foreign),
    },
];
fn subject(c: Input) -> Result<Value, Failure> {
    let first = Arc::new(QueryPlan::default());
    let other = match c {
        Scenario::Same => Arc::clone(&first),
        Scenario::Other => Arc::new(QueryPlan::default()),
    };
    let symbols = Symbols::default();
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let file = FileQueries::open(&first, None, None, &texts).ok_or(())?;
    Ok(file.belongs_to(&other))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject)
}
