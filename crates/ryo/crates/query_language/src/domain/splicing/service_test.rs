//! The bounds of the lowered expansions a compile keeps: a shelf keeps at most `LOWERED_EXPANSIONS` expansions, and
//! `LoweredExpansions` at most `LOWERING_ENVIRONMENTS` shelves; past either, an expansion is lowered and not kept.

use super::constants::{LOWERED_EXPANSIONS, LOWERING_ENVIRONMENTS};
use super::service::{keep, kept_lowering, shelf_for};
use super::types::{ExpansionShelf, LoweredExpansion, LoweredExpansions};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::types::OriginCount;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::program::types::{
    ExpandedDefinitions, FragmentContract, FragmentContractParameter, RelationSignature,
};
use ryo_values::domain::query_syntax::types::ArgumentValue;
use ryo_values::domain::query_tokens::types::{
    ByteOffset, QueryOrigin, QuerySourceId, QuerySpan, StepIdentity, WrittenLiteral,
    WrittenParameter,
};
use std::sync::Arc;

type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_shelf_keeps_lowered_expansions_up_to_its_bound"),
        word: Word::Succeeds,
        input: || a_shelf_keeps_lowered_expansions_up_to_its_bound,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("lowered_expansions_keep_shelves_up_to_their_bound"),
        word: Word::Succeeds,
        input: || lowered_expansions_keep_shelves_up_to_their_bound,
        expect: Expect::Value(()),
    },
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(0),
    end: ByteOffset(1),
};

/// The contract of a call of fragment `query/kinds` that bound `wanted` to `value`.
fn contract(value: usize) -> FragmentContract {
    FragmentContract {
        first: StepIdentity(1),
        result: StepIdentity(1),
        inputs: Vec::new(),
        output: RelationSignature {
            columns: Vec::new(),
            span: SPAN,
        },
        output_span: SPAN,
        origin: QueryOrigin {
            source_id: QuerySourceId("query/kinds".into()),
            span: SPAN,
            trace: [].into(),
        },
        statements: ExpandedDefinitions(1),
        parameters: vec![FragmentContractParameter {
            name: WrittenParameter("wanted".into()),
            value: ArgumentValue::Text(WrittenLiteral(value.to_string().into())),
        }],
    }
}

/// A lowering that made no step.
fn lowered() -> LoweredExpansion {
    LoweredExpansion {
        start: StepId(0),
        origins_before: OriginCount(0),
        steps: Vec::new(),
        origins: Vec::new(),
        traces: vec![Vec::new()],
    }
}

fn a_shelf_keeps_lowered_expansions_up_to_its_bound() {
    let shelf = ExpansionShelf::default();
    for value in 0..=LOWERED_EXPANSIONS {
        assert!(kept_lowering(&shelf, &contract(value)).is_none());
        keep(&shelf, &contract(value), lowered());
    }
    assert!(kept_lowering(&shelf, &contract(0)).is_some());
    assert!(kept_lowering(&shelf, &contract(LOWERED_EXPANSIONS - 1)).is_some());
    assert!(kept_lowering(&shelf, &contract(LOWERED_EXPANSIONS)).is_none());
}

fn lowered_expansions_keep_shelves_up_to_their_bound() {
    let expansions = LoweredExpansions::default();
    let catalogs: Vec<Vec<Relation>> = (0..=LOWERING_ENVIRONMENTS)
        .map(|count| vec![Relation::SyntaxTree; count])
        .collect();
    let shelves: Vec<Arc<ExpansionShelf>> = catalogs
        .iter()
        .map(|catalog| shelf_for(&expansions, catalog, &[], &[], &[]))
        .collect();
    for (catalog, shelf) in catalogs.iter().zip(&shelves) {
        keep(shelf, &contract(0), lowered());
        let again = shelf_for(&expansions, catalog, &[], &[], &[]);
        assert_eq!(
            Arc::ptr_eq(&again, shelf),
            catalog.len() < LOWERING_ENVIRONMENTS
        );
    }
    assert_eq!(
        expansions.0.lock().map(|held| held.len()).unwrap_or(0),
        LOWERING_ENVIRONMENTS
    );
}
