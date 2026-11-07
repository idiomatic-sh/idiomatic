//! What the `typecheck` use case promises, against the fakes of the ports it calls.

use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::{ColumnLabel, HostedColumn, HostedName, HostedRelation};
use ryo_values::domain::observations::types::{
    BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
};
use ryo_values::domain::plan_parts::types::StepScope;
use std::sync::Arc;

use super::errors::TypecheckError;
use super::interface::TypecheckUseCase as _;
use ryo_values::domain::plans::types::QueryRequest;
use super::use_case::TypecheckUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::selects;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::{LetStep, WrittenEmit, WrittenProgram};
use ryo_values::domain::query_syntax::types::{
    Alias, Columns, Condition, Expression, Filter, Joining, QuerySource, Select, SourceReference,
    WrittenOperand, WrittenTest,
};
use ryo_values::domain::query_tokens::types::{
    ByteOffset, QuerySpan, StepName, WrittenLiteral, WrittenRelation,
};

/// What a row gives the use case.
type Input = fn() -> Given;
/// What `typecheck` answers when the text is valid: how many relations the query reads.
type Value = usize;
type Failure = TypecheckError;

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
        name: StepName(
            "functions".into(),
            ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
        ),
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

fn program_with(filters: Vec<Filter>) -> WrittenProgram {
    WrittenProgram {
        linked: ryo_values::domain::query_tokens::types::Linking::Unlinked,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
        observations: Vec::new(),
        emit_origin: ryo_values::domain::query_tokens::types::Attribution::Unlocated,
        parameters: Vec::new(),
        steps: vec![read_function(filters)],
        emit: WrittenEmit {
            step: StepName(
                "functions".into(),
                ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
            ),
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

fn emitting_a_step_nobody_defined() -> Given {
    let mut program = program_with(Vec::new());
    program.emit.step = StepName(
        "later".into(),
        ryo_values::domain::query_tokens::types::StepIdentified::Unassigned,
    );
    Given {
        syntax: SyntaxFake { program },
        ..text_the_fake_reads()
    }
}

/// An external schema supplied at the real inbound API, without supplying any facts.
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

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_distinct_observation_schema_is_admitted"),
        word: Word::Succeeds,
        input: || {
            || Given {
                observations: vec![observed("external", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Value(1),
    },
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
            selects!(matches!(error, TypecheckError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("observation_cannot_shadow_catalog_relation"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("function", &["file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, TypecheckError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("duplicate_observation_sources_are_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![
                    observed("external", &["file"]),
                    observed("external", &["file"]),
                ],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, TypecheckError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("blank_observation_column_is_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &[" "])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, TypecheckError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("duplicate_observation_columns_are_refused"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &["file", "file"])],

                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, TypecheckError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("observation_cannot_shadow_hosted_relation"),
        word: Word::Refuses,
        input: || {
            || Given {
                observations: vec![observed("external", &["file"])],
                hosted: vec![HostedRelation {
                    name: HostedName("external".into()),
                    columns: vec![HostedColumn {
                        name: ColumnLabel("file".into()),
                        kind: CellKind::File,
                    }],
                }],
                ..text_the_fake_reads()
            }
        },
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, TypecheckError::ObservationInputs { .. }))
        }),
    },
    Case {
        name: CaseName("valid_text_is_accepted"),
        word: Word::Succeeds,
        input: || text_the_fake_reads,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("relation_names_are_checked_without_any_language_linked"),
        word: Word::Succeeds,
        input: || text_the_fake_reads,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("syntax_error_is_refused_with_its_span"),
        word: Word::Refuses,
        input: || text_the_fake_refuses,
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, TypecheckError::Located { origin, error }
                if !origin.source_id.0.is_empty()
                    && origin.trace.is_empty()
                    && matches!(error.as_ref(), TypecheckError::Syntax { span, .. } if *span == origin.span)))
        }),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || catalog_without_relations,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, TypecheckError::UnknownRelation { .. })
                    || matches!(error, TypecheckError::Located { error, .. } if matches!(error.as_ref(), TypecheckError::UnknownRelation { .. })),
            )
        }),
    },
    Case {
        name: CaseName("type_mismatch_is_refused"),
        word: Word::Refuses,
        input: || comparing_text_with_a_number,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, TypecheckError::Type { .. })
                    || matches!(error, TypecheckError::Located { error, .. } if matches!(error.as_ref(), TypecheckError::Type { .. })),
            )
        }),
    },
    Case {
        name: CaseName("a_step_nobody_defined_is_refused_as_invalid"),
        word: Word::Refuses,
        input: || emitting_a_step_nobody_defined,
        expect: Expect::Refusal(|error| {
            selects!(
                matches!(error, TypecheckError::Invalid { .. })
                    || matches!(error, TypecheckError::Located { error, .. } if matches!(error.as_ref(), TypecheckError::Invalid { .. })),
            )
        }),
    },
    Case {
        name: CaseName("fails_when_catalog_port_fails"),
        word: Word::Fails,
        input: || failing_catalog,
        expect: Expect::Refusal(|error| selects!(matches!(error, TypecheckError::Catalog { .. }))),
    },
];

fn subject(given: fn() -> Given) -> Result<Value, Failure> {
    let given = given();
    let use_case = TypecheckUseCaseImpl::new(Arc::new(given.syntax), Arc::new(given.catalog));
    use_case
        .typecheck(QueryRequest {
            observations: given.observations,
            text: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
                given.text.into(),
            )),
            hosted: given.hosted,
        })
        .map(|report| report.relations.len())
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
