//! What the `typecheck` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::TypecheckError;
use super::interface::TypecheckUseCase as _;
use super::types::TypecheckRequest;
use super::use_case::TypecheckUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::{
    Alias, ByteOffset, ColumnName, Columns, Condition, Expression, Filter, Joining, LetStep,
    Program, QuerySource, QuerySpan, Select, SourceReference, StepName, WrittenEmit,
    WrittenLiteral, WrittenOperand, WrittenRelation, WrittenTest,
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
        linked: false,
        imports: Vec::new(),
        origins: Vec::new(),
        fragment_contracts: Vec::new(),
        step_origins: std::collections::BTreeMap::new(),
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
    }
}

fn text_the_fake_refuses() -> Given {
    Given {
        syntax: SyntaxFake::default(),
        catalog: CatalogFake::default(),
        text: "let a = ;",
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
    program.emit.step = StepName("later".into(), None);
    Given {
        syntax: SyntaxFake { program },
        ..text_the_fake_reads()
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
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
        expect: Expect::Refusal(|error| selects(matches!(error, TypecheckError::Syntax { .. }))),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || catalog_without_relations,
        expect: Expect::Refusal(|error| {
            selects(
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
            selects(
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
            selects(
                matches!(error, TypecheckError::Invalid { .. })
                    || matches!(error, TypecheckError::Located { error, .. } if matches!(error.as_ref(), TypecheckError::Invalid { .. })),
            )
        }),
    },
    Case {
        name: CaseName("fails_when_catalog_port_fails"),
        word: Word::Fails,
        input: || failing_catalog,
        expect: Expect::Refusal(|error| selects(matches!(error, TypecheckError::Catalog { .. }))),
    },
];

fn subject(given: fn() -> Given) -> Result<Value, Failure> {
    let given = given();
    let use_case = TypecheckUseCaseImpl::new(Arc::new(given.syntax), Arc::new(given.catalog));
    use_case
        .typecheck(TypecheckRequest {
            text: ryo_values::domain::program::types::QueryBundle::from(QueryText(
                given.text.into(),
            )),
            hosted: Vec::new(),
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
