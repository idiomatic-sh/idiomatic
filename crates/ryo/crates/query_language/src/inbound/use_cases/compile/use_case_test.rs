//! What the `compile` use case promises, against the fakes of the ports it calls.

use crate::cases::Case;
use crate::cases::CaseName;
use crate::cases::Expect;
use crate::cases::Word;
use crate::cases::run;
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::selects;
use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::observations::types::BoundObservationInput;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::Alias;
use ryo_values::domain::program::types::ByteOffset;
use ryo_values::domain::program::types::ColumnReference;
use ryo_values::domain::program::types::Columns;
use ryo_values::domain::program::types::Condition;
use ryo_values::domain::program::types::Expression;
use ryo_values::domain::program::types::Filter;
use ryo_values::domain::program::types::Joining;
use ryo_values::domain::program::types::LetStep;
use ryo_values::domain::program::types::Program;
use ryo_values::domain::program::types::QuerySource;
use ryo_values::domain::program::types::QuerySpan;
use ryo_values::domain::program::types::Select;
use ryo_values::domain::program::types::SourceReference;
use ryo_values::domain::program::types::StepName;
use ryo_values::domain::program::types::WrittenEmit;
use ryo_values::domain::program::types::WrittenLiteral;
use ryo_values::domain::program::types::WrittenOperand;
use ryo_values::domain::program::types::WrittenRelation;
use ryo_values::domain::program::types::WrittenTest;
use std::sync::Arc;
use super::errors::CompileError;
use super::interface::CompileUseCase as _;
use super::types::CompileRequest;
use super::use_case::CompileUseCaseImpl;

/// What a row gives the use case.
type Input = fn() -> Given;
/// What a row answers.
type Value = usize;
type Failure = CompileError;

const NOWHERE: QuerySpan = QuerySpan {
    start: ByteOffset(0),
    end: ByteOffset(0),
};

/// What a row gives the use case: the ports it calls, and the text.
struct Given {
    syntax: SyntaxFake,
    catalog: CatalogFake,
    text: &'static str,
    observations: Vec<BoundObservationInput>,
    hosted: Vec<HostedRelation>,
}

fn read_function(filters: Vec<Filter>) -> LetStep {
    LetStep {
        name: StepName("functions".into(), None),
        expression: Expression::Select(Select {
            columns: Columns::Every,
            from: QuerySource {
                reference: SourceReference::Relation(WrittenRelation("function".into())),
                alias: Alias::Unwritten,
                span: NOWHERE,
            },
            join: Joining::Alone,
            filters: filters.into_iter().map(Condition::Test).collect(),
        }),
        span: NOWHERE,
    }
}

fn program_with(filters: Vec<Filter>) -> Program {
    Program {
        linked: ryo_values::domain::program::types::Linking::Unlinked,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: None,
        parameters: Vec::new(),
        steps: vec![read_function(filters)],
        emit: WrittenEmit {
            step: StepName("functions".into(), None),
            subject: ColumnName("file".into()),
            key: vec![ColumnName("span".into())],
            span: NOWHERE,
        },
    }
}

fn text_the_fake_reads() -> Given {
    Given {
        syntax: SyntaxFake {
            program: program_with(Vec::new()),
        },
        catalog: CatalogFake::default(),
        text: "emit",
        observations: Vec::new(),
        hosted: Vec::new(),
    }
}

fn text_the_fake_refuses() -> Given {
    Given {
        syntax: SyntaxFake::default(),
        catalog: CatalogFake::default(),
        text: "let a = ;",
        observations: Vec::new(),
        hosted: Vec::new(),
    }
}

fn catalog_without_relations() -> Given {
    Given {
        catalog: CatalogFake {
            relations: Vec::new(),
            fails: false,
        },
        ..text_the_fake_reads()
    }
}

fn failing_catalog() -> Given {
    Given {
        catalog: CatalogFake {
            relations: Vec::new(),
            fails: true,
        },
        ..text_the_fake_reads()
    }
}

fn comparing_text_with_a_number() -> Given {
    let filter = Filter {
        left: WrittenOperand::Text(WrittenLiteral("a".into())),
        test: WrittenTest::Equal,
        right: WrittenOperand::Number(Integer(1)),
        span: NOWHERE,
    };
    Given {
        syntax: SyntaxFake {
            program: program_with(vec![filter]),
        },
        ..text_the_fake_reads()
    }
}

/// A query that compares the `kind` of `defines` with the text it is given.
fn defines_where_kind_is(word: &str) -> Given {
    let filter = Filter {
        left: WrittenOperand::Column(ColumnReference {
            alias: Alias::Unwritten,
            column: ColumnName("kind".into()),
        }),
        test: WrittenTest::Equal,
        right: WrittenOperand::Text(WrittenLiteral(word.into())),
        span: NOWHERE,
    };
    let mut step = read_function(vec![filter]);
    if let Expression::Select(select) = &mut step.expression {
        select.from.reference = SourceReference::Relation(WrittenRelation("defines".into()));
    }
    Given {
        syntax: SyntaxFake {
            program: Program {
                steps: vec![step],
                ..program_with(Vec::new())
            },
        },
        catalog: CatalogFake {
            relations: vec![Relation::Defines],
            fails: false,
        },
        text: "emit",
        hosted: vec![],
        observations: vec![],
    }
}

fn a_kind_of_defines_that_is_misspelled() -> Given {
    defines_where_kind_is("headline")
}

fn a_kind_of_defines_that_exists() -> Given {
    defines_where_kind_is("section")
}

fn emitting_a_step_nobody_defined() -> Given {
    let mut program = program_with(Vec::new());
    program.emit.step = StepName("later".into(), None);
    Given {
        syntax: SyntaxFake { program },
        ..text_the_fake_reads()
    }
}

fn same_text_twice() -> Given {
    text_the_fake_reads()
}

/// An external schema supplied at the real inbound API, without supplying any facts.

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
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
        name: CaseName("a_word_that_is_no_kind_of_the_column_it_is_compared_with_is_refused"),
        word: Word::Refuses,
        input: || a_kind_of_defines_that_is_misspelled,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(
                    error,
                    CompileError::Type { message, .. }
                        if message == "`headline` is not a kind of defines; the kinds are function, method, class, struct, enum, enum-member, trait, interface, type, type-alias, type-parameter, namespace, package, import, variable, constant, parameter, property, macro, section, label, implementation"
                ) || matches!(error, CompileError::Located { error, .. }
                    if matches!(error.as_ref(), CompileError::Type { message, .. }
                        if message == "`headline` is not a kind of defines; the kinds are function, method, class, struct, enum, enum-member, trait, interface, type, type-alias, type-parameter, namespace, package, import, variable, constant, parameter, property, macro, section, label, implementation")),
            )
        }),
    },
    Case {
        name: CaseName("a_word_that_is_a_kind_of_the_column_compiles"),
        word: Word::Succeeds,
        input: || a_kind_of_defines_that_exists,
        expect: Expect::Value(1),
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

fn subject(given: fn() -> Given) -> Result<Value, Failure> {
    let given = given();
    let use_case = CompileUseCaseImpl::new(Arc::new(given.syntax), Arc::new(given.catalog));
    let request = CompileRequest {
        observations: given.observations,
        text: ryo_values::domain::program::types::QueryBundle::from(QueryText(given.text.into())),
        hosted: given.hosted,
    };
    let first = use_case.compile(request.clone())?;
    let second = use_case.compile(request)?;
    assert_eq!(first, second, "compilation is deterministic");
    Ok(first.plan.steps.len())
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
mod traversing;
