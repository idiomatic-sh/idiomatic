//! What the `compile` use case promises, against the fakes of the ports it calls.

use std::sync::Arc;

use super::errors::CompileError;
use super::interface::CompileUseCase as _;
use super::types::CompileRequest;
use super::use_case::CompileUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use ryo_language::domain::relations::types::Integer;
use ryo_language::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::program::types::{
    Alias, ByteOffset, ColumnName, ColumnReference, Columns, Condition, Expression, Filter,
    Joining, LetStep, Program, QuerySource, QuerySpan, Select, SourceReference, StepName,
    WrittenEmit, WrittenLiteral, WrittenOperand, WrittenRelation, WrittenTest,
};

/// What a row gives the use case.
type Input = fn() -> Given;
/// What `compile` answers: how many steps the plan has.
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
}

fn read_function(filters: Vec<Filter>) -> LetStep {
    LetStep {
        name: StepName("functions".into()),
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
        parameters: Vec::new(),
        steps: vec![read_function(filters)],
        emit: WrittenEmit {
            step: StepName("functions".into()),
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
    program.emit.step = StepName("later".into());
    Given {
        syntax: SyntaxFake { program },
        ..text_the_fake_reads()
    }
}

fn same_text_twice() -> Given {
    text_the_fake_reads()
}

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
        expect: Expect::Refusal(|error| selects(matches!(error, CompileError::Syntax { .. }))),
    },
    Case {
        name: CaseName("unknown_relation_is_refused"),
        word: Word::Refuses,
        input: || catalog_without_relations,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, CompileError::UnknownRelation { .. }))
        }),
    },
    Case {
        name: CaseName("type_mismatch_is_refused"),
        word: Word::Refuses,
        input: || comparing_text_with_a_number,
        expect: Expect::Refusal(|error| selects(matches!(error, CompileError::Type { .. }))),
    },
    Case {
        name: CaseName("a_word_that_is_no_kind_of_the_column_it_is_compared_with_is_refused"),
        word: Word::Refuses,
        input: || a_kind_of_defines_that_is_misspelled,
        expect: Expect::Refusal(|error| {
            selects(matches!(
                error,
                CompileError::Type { message, .. }
                    if message == "`headline` is not a kind of defines; the kinds are function, method, class, struct, enum, enum-member, trait, interface, type, type-alias, type-parameter, namespace, package, import, variable, constant, parameter, property, macro, section, label, implementation"
            ))
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
        expect: Expect::Refusal(|error| selects(matches!(error, CompileError::Invalid { .. }))),
    },
    Case {
        name: CaseName("fails_when_catalog_port_fails"),
        word: Word::Fails,
        input: || failing_catalog,
        expect: Expect::Refusal(|error| selects(matches!(error, CompileError::Catalog { .. }))),
    },
];

fn subject(given: fn() -> Given) -> Result<Value, Failure> {
    let given = given();
    let use_case = CompileUseCaseImpl::new(Arc::new(given.syntax), Arc::new(given.catalog));
    let request = CompileRequest {
        text: QueryText(given.text.into()),
        hosted: Vec::new(),
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
