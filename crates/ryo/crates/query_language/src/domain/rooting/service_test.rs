//! The root of a bundle is given identities for its steps and an origin for its emit.

use super::service::assign_root_identities;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::program::types::{LetStep, WrittenEmit, WrittenProgram};
use ryo_values::domain::query_syntax::types::{
    Alias, Columns, Expression, Joining, QuerySource, Select, SourceReference,
};
use ryo_values::domain::query_tokens::types::{
    Attribution, ByteOffset, Linking, QuerySpan, StepIdentified, StepName, WrittenRelation,
};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A root whose steps carry no identity.
    UnidentifiedSteps,
}

/// Whether every step of the root has an identity.
type Value = bool;
type Failure = std::convert::Infallible;
/// What a row asks of the service.
type Input = Scenario;

const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(3),
    end: ByteOffset(11),
};

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_step_of_the_root_is_given_an_identity"),
    word: Word::Succeeds,
    input: || Scenario::UnidentifiedSteps,
    expect: Expect::Value(true),
}];

fn step(name: &str) -> LetStep {
    LetStep {
        name: StepName(name.into(), StepIdentified::Unassigned),
        expression: Expression::Select(Select {
            columns: Columns::Every,
            from: QuerySource {
                reference: SourceReference::Relation(WrittenRelation("function".into())),
                alias: Alias::Unwritten,
                span: SPAN,
            },
            join: Joining::Alone,
            filters: Vec::new(),
        }),
        span: SPAN,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let Scenario::UnidentifiedSteps = scenario;
    let mut program = WrittenProgram {
        linked: Linking::Unlinked,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: Attribution::Unlocated,
        parameters: Vec::new(),
        steps: vec![step("one"), step("two")],
        emit: WrittenEmit {
            step: StepName("two".into(), StepIdentified::Unassigned),
            subject: ColumnName("file".into()),
            key: vec![ColumnName("span".into())],
            span: SPAN,
        },
    };
    assign_root_identities(&mut program);
    Ok(program
        .steps
        .iter()
        .all(|step| matches!(step.name.1, StepIdentified::Assigned(_))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
