//! The `syntax` service's cases, on values alone.

use super::service::{operators_used, relations_read, shape_named};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::program::types::{Alias, ByteOffset, Call, Columns, Expression, Joining, LetStep, OperationName, WrittenProgram, QuerySource, QuerySpan, Select, SourceReference, StepName, WrittenEmit, WrittenRelation};
use ryo_values::domain::descriptions::types::ColumnName;

/// A scenario: what a row asks of a program that reads one relation and calls `project`.
type Input = fn(&WrittenProgram) -> usize;
/// What a scenario answers: how many things it found.
type Value = usize;
type Failure = ();

fn nowhere() -> QuerySpan {
    QuerySpan {
        start: ByteOffset(0),
        end: ByteOffset(0),
    }
}

fn program() -> WrittenProgram {
    let read_function = LetStep {
        name: StepName("functions".into(), None),
        expression: Expression::Select(Select {
            columns: Columns::Every,
            from: QuerySource {
                reference: SourceReference::Relation(WrittenRelation("function".into())),
                alias: Alias::Unwritten,
                span: nowhere(),
            },
            join: Joining::Alone,
            filters: Vec::new(),
        }),
        span: nowhere(),
    };
    let project = LetStep {
        name: StepName("paths".into(), None),
        expression: Expression::Call(Call {
            name: OperationName("project".into()),
            arguments: Vec::new(),
            span: nowhere(),
        }),
        span: nowhere(),
    };
    WrittenProgram {
        linked: false,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: ryo_values::domain::program::types::EmitOrigin::Unlocated,
        parameters: Vec::new(),
        steps: vec![read_function, project],
        emit: WrittenEmit {
            step: StepName("paths".into(), None),
            subject: ColumnName("path".into()),
            key: Vec::new(),
            span: nowhere(),
        },
    }
}

fn relations_counted(program: &WrittenProgram) -> usize {
    relations_read(program).len()
}

fn operators_counted(program: &WrittenProgram) -> usize {
    operators_used(program).len()
}

fn project_shape_found(_program: &WrittenProgram) -> usize {
    usize::from(shape_named(&OperationName("project".into())).is_some())
}

fn unknown_shape_found(_program: &WrittenProgram) -> usize {
    usize::from(shape_named(&OperationName("sort".into())).is_some())
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_program_reads_each_relation_once"),
        word: Word::Succeeds,
        input: || relations_counted,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_program_uses_select_and_project"),
        word: Word::Succeeds,
        input: || operators_counted,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("project_is_an_operation"),
        word: Word::Succeeds,
        input: || project_shape_found,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("sort_is_not_an_operation"),
        word: Word::Succeeds,
        input: || unknown_shape_found,
        expect: Expect::Value(0),
    },
];

fn subject(scenario: Input) -> Value {
    scenario(&program())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
