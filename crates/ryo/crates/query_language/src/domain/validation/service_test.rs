//! Validation consumes typed syntax and catalog values without calling a parser or port.

use super::errors::ValidationError;
use super::service::validate_with_observations;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::selects;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::observations::types::ObservationName;
use ryo_values::domain::program::types::{
    LetStep, WrittenEmit, WrittenObservation, WrittenProgram,
};
use ryo_values::domain::query_syntax::types::{
    Alias, Call, Columns, Condition, Expression, Filter, Join, Joining, QuerySource, Select,
    SourceReference, WrittenOperand, WrittenTest,
};
use ryo_values::domain::query_tokens::types::{
    ByteOffset, OperationName, QueryOrigin, QuerySourceId, QuerySpan, StepIdentity, StepName,
    WrittenLiteral, WrittenParameter, WrittenRelation,
};

#[derive(Clone, Copy)]
enum Scenario {
    Valid,
    UnknownRelation,
    MissingEmitStep,
    DuplicateStep,
    BlankObservation,
    DuplicateObservation,
    UndeclaredParameter,
    UnknownOperation,
    MissingArgument,
    JoinWithoutColumns,
    LocatedStep,
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
    Case {
        name: CaseName("undeclared_parameter_keeps_name_and_span"),
        word: Word::Refuses,
        input: || Scenario::UndeclaredParameter,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ValidationError::UnknownParameter { span, .. } if *span == SPAN)
            )
        }),
    },
    Case {
        name: CaseName("unknown_operation_keeps_name_and_span"),
        word: Word::Refuses,
        input: || Scenario::UnknownOperation,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, ValidationError::UnknownOperation { span, .. } if *span == SPAN)
            )
        }),
    },
    Case {
        name: CaseName("call_without_required_argument_keeps_span"),
        word: Word::Refuses,
        input: || Scenario::MissingArgument,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::MissingArgument { .. }))
        }),
    },
    Case {
        name: CaseName("join_of_every_column_keeps_span"),
        word: Word::Refuses,
        input: || Scenario::JoinWithoutColumns,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, ValidationError::JoinNeedsColumns { span } if *span == SPAN))
        }),
    },
    Case {
        name: CaseName("step_error_names_the_origin_of_its_fragment"),
        word: Word::Refuses,
        input: || Scenario::LocatedStep,
        expect: Expect::Refusal(|error| selects!(matches!(error, ValidationError::Located { .. }))),
    },
];

fn source(relation: &str) -> LetStep {
    LetStep {
        name: StepName(
            "rows".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
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

/// A step that reads the relation `function` and keeps the rows `filter` keeps.
fn filtered(filter: Filter) -> LetStep {
    let mut step = source("function");
    if let Expression::Select(select) = &mut step.expression {
        select.filters = vec![Condition::Test(filter)];
    }
    step
}

/// A step that is the call of the operation `name`, given no arguments.
fn called(name: &str) -> LetStep {
    LetStep {
        name: StepName(
            "rows".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
        expression: Expression::Call(Call {
            name: OperationName(name.into()),
            arguments: Vec::new(),
            span: SPAN,
        }),
        span: SPAN,
    }
}

fn observation(name: &str) -> WrittenObservation {
    WrittenObservation {
        step: StepName(
            "rows".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
        name: ObservationName(name.into()),
        span: SPAN,
        origin: ryo_values::domain::query_tokens::types::Attribution::Unlocated,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let mut program = WrittenProgram {
        linked: ryo_values::domain::query_tokens::types::Linking::Unlinked,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: ryo_values::domain::query_tokens::types::Attribution::Unlocated,
        parameters: Vec::new(),
        steps: vec![source("function")],
        emit: WrittenEmit {
            step: StepName(
                "rows".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
            ),
            subject: ColumnName("file".into()),
            key: vec![ColumnName("span".into())],
            span: SPAN,
        },
    };
    match scenario {
        Scenario::Valid => {}
        Scenario::UnknownRelation => program.steps = vec![source("nothing")],
        Scenario::MissingEmitStep => {
            program.emit.step = StepName(
                "missing".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
            )
        }
        Scenario::UndeclaredParameter => {
            program.steps = vec![filtered(Filter {
                left: WrittenOperand::Parameter(WrittenParameter("limit".into())),
                test: WrittenTest::Equal,
                right: WrittenOperand::Text(WrittenLiteral("x".into())),
                span: SPAN,
            })];
        }
        Scenario::UnknownOperation => program.steps = vec![called("nothing")],
        Scenario::MissingArgument => program.steps = vec![called("span-bounds")],
        Scenario::JoinWithoutColumns => {
            let mut joined = source("function");
            if let Expression::Select(select) = &mut joined.expression {
                select.join = Joining::Joined(Join {
                    source: select.from.clone(),
                    on: Vec::new(),
                });
            }
            program.steps = vec![joined];
        }
        Scenario::LocatedStep => {
            let identity = StepIdentity(1);
            let mut unknown = source("nothing");
            unknown.name = StepName(
                "rows".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Assigned(identity),
            );
            program.steps = vec![unknown];
            program.step_origins.insert(
                identity,
                QueryOrigin {
                    source_id: QuerySourceId("module".into()),
                    span: SPAN,
                    trace: Vec::new(),
                },
            );
        }
        Scenario::DuplicateStep => program.steps.push(source("function")),
        Scenario::BlankObservation => program.observations.push(observation(" \t")),
        Scenario::DuplicateObservation => {
            program.observations = vec![observation("exported"), observation("exported")]
        }
    }
    validate_with_observations(&program, &[Relation::Function], &[], &[])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
