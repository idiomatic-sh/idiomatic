//! What the `compile` use case promises, against the fakes of the ports it calls.

use super::errors::CompileError;
use super::fixtures::{
    Given, catalog_without_relations, comparing_text_with_a_number, emitting_a_step_nobody_defined,
    failing_catalog, same_text_twice, text_the_fake_refuses, text_the_fake_reads,
};
use super::interface::CompileUseCase;
use ryo_values::domain::plans::types::QueryRequest;
use super::use_case::CompileUseCaseImpl;
use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::winnow::winnow;
use crate::selects;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
};
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plan_parts::types::StepScope;
use std::sync::Arc;

/// What a row gives the use case.
type Input = fn() -> Given;
/// What a row answers.
type Value = usize;
type Failure = CompileError;

/// The functions of each name counted by the files they are in: only the name and the file are asked of the rows.
const DISTINCT_COUNT: &str = "let f = select * from \"function\"; let c = group-count(from=f, keys=[\"name\"], count=\"n\", distinct=\"file\"); emit c subject name key(n);";
/// The operations of a plan that only projects.
const SELECT_PROJECT: &str = "select project";
/// The operations of a plan that selects a closure.
const SELECT_CLOSURE: &str = "select closure";
/// The operation of a plan that walks edges depth first.
const ORDERED_DFS: &str = "ordered-dfs";
/// The operations of a plan that counts groups by distinct values, with the rows cut to the columns asked.
const SELECT_GROUP_COUNT_STAR: &str = "select group-count*";

/// An external schema supplied at the real inbound API, without supplying any facts.

/// The rows of this part's cases.
/// An observation input named `name` with the given file-kind columns, shared by the observation groups.
fn observed(name: &str, columns: &[&str]) -> BoundObservationInput {
    BoundObservationInput {
        name: ObservationBindingName(name.into()),
        columns: columns
            .iter()
            .map(|name| ObservationColumn {
                name: ObservationColumnName((*name).into()),
                kind: CellKind::File,
            })
            .collect(),
        scope: StepScope::PerFile,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("blank_observation_name_is_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed(" ", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("compiles_valid_text_to_a_plan"),
        word: Word::Succeeds,
        input: || text_the_fake_reads,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("same_text_gives_the_same_plan"),
        word: Word::Succeeds,
        input: || same_text_twice,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("invalid_text_is_refused"),
        word: Word::Refuses,
        input: || text_the_fake_refuses,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, CompileError::Located { origin, error }
                if !origin.source_id.0.is_empty()
                    && origin.trace.is_empty()
                    && matches!(error.as_ref(), CompileError::Syntax { span, .. } if *span == origin.span)))
        }),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || catalog_without_relations,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, CompileError::UnknownRelation { .. })
                    || matches!(error, CompileError::Located { error, .. } if matches!(error.as_ref(), CompileError::UnknownRelation { .. })),
            )
        }),
    },
    Case {
        name: CaseName("type_mismatch_is_refused"),
        word: Word::Refuses,
        input: || comparing_text_with_a_number,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, CompileError::Type { .. })
                    || matches!(error, CompileError::Located { error, .. } if matches!(error.as_ref(), CompileError::Type { .. })),
            )
        }),
    },
    Case {
        name: CaseName("a_step_nobody_defined_is_refused_as_invalid"),
        word: Word::Refuses,
        input: || emitting_a_step_nobody_defined,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, CompileError::Invalid { .. })
                    || matches!(error, CompileError::Located { error, .. } if matches!(error.as_ref(), CompileError::Invalid { .. })),
            )
        }),
    },
    Case {
        name: CaseName("fails_when_catalog_port_fails"),
        word: Word::Fails,
        input: || failing_catalog,
        expect: Expect::Refusal(|error| selects!(matches!(error, CompileError::Catalog { .. }))),
    },
];

/// The compile use case over the real parser and a catalog that holds `relations`.
fn compiler_over(relations: Vec<Relation>) -> impl CompileUseCase {
    CompileUseCaseImpl::new(
        Arc::new(winnow()),
        Arc::new(CatalogFake {
            relations,
            fails: false,
        }),
    )
}

/// The compile use case over the real parser and the catalog of every relation the language reads.
fn compiler_over_the_language() -> impl CompileUseCase {
    CompileUseCaseImpl::new(
        Arc::new(winnow()),
        Arc::new(crate::outbound::catalog::factory::language()),
    )
}

/// What the use case answers, twice, for what a row gives it: how many steps the plan has.
fn compile_twice(given: Given) -> Result<Value, Failure> {
    let use_case = CompileUseCaseImpl::new(Arc::new(given.syntax), Arc::new(given.catalog));
    let request = QueryRequest {
        observations: given.observations,
        text: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
            given.text.into(),
        )),
        hosted: given.hosted,
        modules: ryo_values::domain::program::types::ParsedModules::default(),
    };
    let first = use_case.compile(request.clone())?;
    let second = use_case.compile(request)?;
    assert_eq!(first, second, "compilation is deterministic");
    Ok(first.plan.steps.len())
}

fn subject(given: fn() -> Given) -> Result<Value, Failure> {
    compile_twice(given())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

/// A file-key join uses the generic executor, which needs a separate predicate step.
#[cfg(test)]
mod fragment_contracts;
#[cfg(test)]
mod observation_plans;
#[cfg(test)]
mod observations;

#[cfg(test)]
mod defines_kinds;

#[cfg(test)]
mod fragment_hygiene;
#[cfg(test)]
mod fragment_refusal;

#[cfg(test)]
mod validation;

#[cfg(test)]
mod assembling;

#[cfg(test)]
mod calling;

#[cfg(test)]
mod combining;

#[cfg(test)]
mod comparing;

#[cfg(test)]
mod decoded_columns;

#[cfg(test)]
mod idioms;

#[cfg(test)]
mod joining;

#[cfg(test)]
mod planning;

#[cfg(test)]
mod reshaping;

#[cfg(test)]
mod resolving;

#[cfg(test)]
mod selecting;

#[cfg(test)]
mod substituting;

#[cfg(test)]
mod spans;

#[cfg(test)]
mod traversing;

#[cfg(test)]
mod closures_and_axes;
