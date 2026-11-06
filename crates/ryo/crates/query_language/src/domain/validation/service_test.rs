//! Validation consumes typed syntax and catalog values without calling a parser or port.

use super::errors::ValidationError;
use super::service::validate;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::program::types::{
    Alias, ByteOffset, ColumnName, Columns, Expression, Joining, LetStep, Program, QuerySource,
    QuerySpan, Select, SourceReference, StepName, WrittenEmit, WrittenObservation, WrittenRelation,
};

#[derive(Clone, Copy)]
enum Scenario {
    Valid,
    UnknownRelation,
    MissingEmitStep,
    DuplicateStep,
    BlankObservation,
    DuplicateObservation,
}
type Input = Scenario;
type Value = ();
type Failure = ValidationError;
const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(7),
    end: ByteOffset(19),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("typed_source_and_emit_are_admitted"),
        word: Word::Succeeds,
        input: || Scenario::Valid,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("unknown_relation_keeps_name_and_span"),
        word: Word::Refuses,
        input: || Scenario::UnknownRelation,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ValidationError::UnknownRelation { name, span } if name == "nothing" && *span == SPAN),
            )
        }),
    },
    Case {
        name: CaseName("missing_emit_step_keeps_name_and_span"),
        word: Word::Refuses,
        input: || Scenario::MissingEmitStep,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ValidationError::UnknownStep { name, span } if name == "missing" && *span == SPAN),
            )
        }),
    },
    Case {
        name: CaseName("duplicate_step_keeps_name_and_span"),
        word: Word::Refuses,
        input: || Scenario::DuplicateStep,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ValidationError::DuplicateStep { name, span } if name == "rows" && *span == SPAN),
            )
        }),
    },
    Case {
        name: CaseName("blank_observation_keeps_span"),
        word: Word::Refuses,
        input: || Scenario::BlankObservation,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::BlankObservation { span } if *span == SPAN))
        }),
    },
    Case {
        name: CaseName("duplicate_observation_keeps_name_and_span"),
        word: Word::Refuses,
        input: || Scenario::DuplicateObservation,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ValidationError::DuplicateObservation { name, span } if name == "exported" && *span == SPAN),
            )
        }),
    },
];

fn source(relation: &str) -> LetStep {
    LetStep {
        name: StepName("rows".into(), None),
        expression: Expression::Select(Select {
            columns: Columns::Every,
            from: QuerySource {
                reference: SourceReference::Relation(WrittenRelation(relation.into())),
                alias: Alias::Unwritten,
                span: SPAN,
            },
            join: Joining::Alone,
            filters: Vec::new(),
        }),
        span: SPAN,
    }
}

fn observation(name: &str) -> WrittenObservation {
    WrittenObservation {
        step: StepName("rows".into(), None),
        name: ObservationName(name.into()),
        span: SPAN,
        origin: None,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut program = Program {
        linked: false,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: None,
        parameters: Vec::new(),
        steps: vec![source("function")],
        emit: WrittenEmit {
            step: StepName("rows".into(), None),
            subject: ColumnName("file".into()),
            key: vec![ColumnName("span".into())],
            span: SPAN,
        },
    };
    match scenario {
        Scenario::Valid => {}
        Scenario::UnknownRelation => program.steps = vec![source("nothing")],
        Scenario::MissingEmitStep => program.emit.step = StepName("missing".into(), None),
        Scenario::DuplicateStep => program.steps.push(source("function")),
        Scenario::BlankObservation => program.observations.push(observation(" \t")),
        Scenario::DuplicateObservation => {
            program.observations = vec![observation("exported"), observation("exported")]
        }
    }
    validate(&program, &[Relation::Function], &[])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
