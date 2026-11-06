//! Modules are checked for repeated step names and import aliases before any call is expanded.

use super::service::{imports_for, validate_step_names};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::linking::errors::LinkingError;
use crate::selects;
use ryo_values::domain::program::types::{
    Alias, ByteOffset, Columns, Expression, ImportDeclaration, Joining, LetStep, QuerySource,
    QuerySourceId, QuerySpan, Select, SourceReference, StepIdentified, StepName, WrittenLiteral,
    WrittenName, WrittenRelation,
};

/// What a row asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// Two steps with different names.
    DistinctSteps,
    /// Two steps with one name.
    RepeatedStep,
    /// Two imports under one alias.
    RepeatedAlias,
}

type Value = ();
type Failure = LinkingError;
/// What a row asks of the service.
type Input = Scenario;

const SPAN: QuerySpan = QuerySpan {
    start: ByteOffset(3),
    end: ByteOffset(11),
};

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("steps_with_different_names_are_admitted"),
        word: Word::Succeeds,
        input: || Scenario::DistinctSteps,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_step_name_written_twice_is_refused_at_the_second_step"),
        word: Word::Refuses,
        input: || Scenario::RepeatedStep,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LinkingError::Located { origin, .. } if origin.span == SPAN))
        }),
    },
    Case {
        name: CaseName("an_alias_written_twice_is_refused_at_the_second_import"),
        word: Word::Refuses,
        input: || Scenario::RepeatedAlias,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, LinkingError::Located { origin, .. } if origin.span == SPAN))
        }),
    },
];

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

fn import(module: &str) -> ImportDeclaration {
    ImportDeclaration {
        alias: WrittenName("same".into()),
        module: WrittenLiteral(module.into()),
        span: SPAN,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = QuerySourceId(Box::from("root"));
    match scenario {
        Scenario::DistinctSteps => validate_step_names(&root, &[step("one"), step("two")]),
        Scenario::RepeatedStep => validate_step_names(&root, &[step("one"), step("one")]),
        Scenario::RepeatedAlias => {
            imports_for(&root, &[import("one"), import("two")]).map(|_| ())
        }
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
