//! Logical set lifetimes keep bag scopes and deferred errors explicit.
use super::super::types::WorkspaceLifetime as Lifetime;
use super::super::workspace::{service::begin, service::prepare};
use crate::domain::{
    operating::errors::OperatingError,
    operator::contract::types::{PrimitiveWorkspace, WorkspaceProgram},
    table_dedup::service::first_time,
    table_model::types::Addition,
};
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
/// What was begun on a fresh workspace before its logical set is read.
#[derive(Clone, Copy, Debug)]
enum Begun {
    /// Nothing.
    Nothing,
    /// A first-occurrence program for the invocation, whose kinds are the fault `ShapeMismatch`.
    FaultyKinds,
    /// A reach-less join's own workspace program, for a primary row.
    ReachlessJoin,
}
/// What a row asks of a workspace.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// Whether the number 1 is new each time it is added to the set of a first-occurrence program of this lifetime, the program
    /// begun for that lifetime, then for a primary row, then for that lifetime again.
    Rows(Lifetime),
    /// The logical set's schema after this was begun.
    Begin(Begun),
}
/// What a workspace answers.
#[derive(Debug, PartialEq)]
enum Answer {
    /// Whether each addition was new.
    Additions([Addition; 3]),
    /// The set's schema is ready.
    Ready,
    /// The workspace holds no logical set.
    NoSet,
}
type Input = Ask;
type Value = Answer;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("invocation_set_survives_rows_and_resets_for_retry"),
        word: Word::Succeeds,
        input: || Ask::Rows(Lifetime::Invocation),
        expect: Expect::Value(Answer::Additions([
            Addition::Added,
            Addition::Existing,
            Addition::Added,
        ])),
    },
    Case {
        name: CaseName("reach_set_restarts_for_each_primary_row"),
        word: Word::Succeeds,
        input: || Ask::Rows(Lifetime::PrimaryRow),
        expect: Expect::Value(Answer::Additions([Addition::Added; 3])),
    },
    Case {
        name: CaseName("unprepared_logical_set_fails_closed"),
        word: Word::Refuses,
        input: || Ask::Begin(Begun::Nothing),
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("unused_schema_fault_remains_pending"),
        word: Word::Refuses,
        input: || Ask::Begin(Begun::FaultyKinds),
        expect: Expect::Error(OperatingError::ShapeMismatch),
    },
    Case {
        name: CaseName("a_join_without_reach_keeps_no_workspace"),
        word: Word::Refuses,
        input: || Ask::Begin(Begun::ReachlessJoin),
        expect: Expect::Error(OperatingError::NotRun),
    },
];
/// A reach-less join's own workspace program.
fn reachless_join() -> WorkspaceProgram {
    use ryo_relations::domain::relations::types::Relation;
    use ryo_values::domain::{
        plan_parts::types::{ColumnIndex, Input as PlanInput},
        plans::types::Operation,
    };
    let join = Operation::Join {
        left: PlanInput::Relation(Relation::SyntaxTree),
        right: PlanInput::Relation(Relation::SyntaxTree),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: Vec::new(),
        filter: Vec::new(),
        view: Vec::new(),
        reach: None,
    };
    prepare(&join, &[], &Ok(vec![CellKind::Number]))
}
fn begun(what: Begun) -> Result<Value, Failure> {
    let mut workspace = PrimitiveWorkspace::default();
    match what {
        Begun::Nothing => {}
        Begun::FaultyKinds => begin(
            &WorkspaceProgram::FirstOccurrences {
                lifetime: Lifetime::Invocation,
                kinds: Err(OperatingError::ShapeMismatch),
            },
            Lifetime::Invocation,
            &mut workspace,
        ),
        Begun::ReachlessJoin => begin(&reachless_join(), Lifetime::PrimaryRow, &mut workspace),
    }
    match workspace.distinct.as_ref() {
        None => Ok(Answer::NoSet),
        Some(state) => state.schema.clone().map(|()| Answer::Ready),
    }
}
fn rows(lifetime: Lifetime) -> Result<Value, Failure> {
    let program = WorkspaceProgram::FirstOccurrences {
        lifetime,
        kinds: Ok(vec![CellKind::Number]),
    };
    let mut workspace = PrimitiveWorkspace::default();
    let mut found = [Addition::Existing; 3];
    for (each, begun) in found
        .iter_mut()
        .zip([lifetime, Lifetime::PrimaryRow, lifetime])
    {
        begin(&program, begun, &mut workspace);
        let Some(state) = workspace.distinct.as_mut() else {
            return Ok(Answer::NoSet);
        };
        state.schema.clone()?;
        *each = first_time(
            &mut state.rows,
            &mut state.seen,
            &[Cell::Number(Integer(1))],
        );
    }
    Ok(Answer::Additions(found))
}
fn subject(ask: Input) -> Result<Value, Failure> {
    match ask {
        Ask::Rows(lifetime) => rows(lifetime),
        Ask::Begin(what) => begun(what),
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
