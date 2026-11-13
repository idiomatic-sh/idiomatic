//! Preflight precedence of producer failures, stored prefixes and registered kinds through the mandatory public execution owner.
use super::super::super::fixtures;
use super::super::admission_order::test::{Answer, Refusal, observed, operation, population};
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_domain::domain::operator::contract::service::semantics;
use ryo_execution_domain::domain::operator::contract::types::PreflightOrder;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::Relation;
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::rows::errors::RowsError;
use std::borrow::Cow;
use std::sync::atomic::{AtomicUsize, Ordering};

/// One precedence question put to the public owner.
#[derive(Clone, Copy, Debug)]
enum Precedence {
    /// A projection over a stored prefix whose last column is ragged.
    StoredPrefix { bad_layout: bool },
    /// Every registered kind's declared preflight policy.
    Declarations,
}
/// What a precedence question answered.
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    Rows(Answer),
    Policies(Cow<'static, [(OperationKind, PreflightOrder)]>),
}
type Input = Precedence;
type Value = Observed;
type Failure = Refusal;
fn stored_prefix(bad_layout: bool) -> Result<Value, Failure> {
    use ryo_relations::domain::relations::service::to_bits;
    use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
    let mut symbols = Symbols::default();
    let symbol = intern_derived(&mut symbols, ByteSlice(b""), NameText("prefix"));
    let table = population(Relation::Function, false, symbol);
    let mut columns = (0..table.kinds.len())
        .map(|column| {
            FactColumn(
                (0..table.count.0)
                    .map(|row| to_bits(table.cells[row * table.kinds.len() + column]).0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let _tail = columns.last_mut().map(|column| column.0.pop());
    let reads = [AtomicUsize::new(0), AtomicUsize::new(0)];
    let read = |relation| {
        let side = usize::from(relation != Relation::Function);
        let _reads = reads[side].fetch_add(1, Ordering::Relaxed);
        Ok(ColumnsView {
            kinds: kinds(Relation::Function),
            columns: &columns,
        })
    };
    let mut facts = fixtures::facts::ReadFacts::new(read);
    let (engine, prepared) = fixtures::requests::prepared(
        inputs::plan(vec![operation(OperationKind::Project, bad_layout)]),
        vec![],
        vec![],
    )
    .expect("a structurally valid request prepares");
    let result = fixtures::requests::run_prepared(
        &engine,
        &prepared,
        b"",
        &Elements::default(),
        &mut symbols,
        &mut facts,
    );
    observed(
        result,
        [
            reads[0].load(Ordering::Relaxed),
            reads[1].load(Ordering::Relaxed),
        ],
    )
    .map(Observed::Rows)
}
fn declarations() -> Result<Value, Failure> {
    Ok(Observed::Policies(
        OperationKind::ALL
            .iter()
            .map(|&kind| (kind, semantics(kind).contract.preflight))
            .collect(),
    ))
}
const fn rows(rows: usize, nodes: &'static [u32], reads: [usize; 2]) -> Expect<Value, Failure> {
    Expect::Value(Observed::Rows(Answer {
        rows,
        nodes: Cow::Borrowed(nodes),
        reads,
    }))
}
const fn refused(error: OperatingError, reads: [usize; 2]) -> Expect<Value, Failure> {
    Expect::Error(Refusal { error, reads })
}
const POLICIES: &[(OperationKind, PreflightOrder)] = {
    use OperationKind::*;
    use PreflightOrder::{ResourcesFirst as R, SchemaFirst as S};
    &[
        (Replace, R),
        (OrderedMatch, R),
        (Select, S),
        (Project, S),
        (Extend, R),
        (Join, S),
        (Union, S),
        (Intersection, S),
        (Difference, S),
        (Antijoin, R),
        (GroupCount, R),
        (GroupDigest, R),
        (Capture, R),
        (Rank, R),
        (Words, R),
        (Axis, R),
        (Closure, R),
        (Cycles, R),
        (Rows, R),
        (Hosted, R),
        (SpanBetween, R),
        (SourceMatches, R),
        (RenderSyntax, R),
        (FitsRendered, R),
        (SpanBounds, R),
        (OrderedDfs, R),
    ]
};
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("readable_stored_prefix_preserves_schema_precedence_and_actual_rows"),
        word: Word::Succeeds,
        input: || Precedence::StoredPrefix { bad_layout: false },
        expect: rows(1, &[1], [4, 0]),
    },
    Case {
        name: CaseName("a_stored_prefix_layout_refusal_commits_no_provider_reread"),
        word: Word::Refuses,
        input: || Precedence::StoredPrefix { bad_layout: true },
        expect: refused(
            OperatingError::Table(RowsError::NoSuchColumn { index: 99 }),
            [1, 0],
        ),
    },
    Case {
        name: CaseName("every_registered_kind_has_an_explicit_independent_preflight_policy"),
        word: Word::Succeeds,
        input: || Precedence::Declarations,
        expect: Expect::Value(Observed::Policies(Cow::Borrowed(POLICIES))),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Precedence::StoredPrefix { bad_layout } => stored_prefix(bad_layout),
        Precedence::Declarations => declarations(),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
