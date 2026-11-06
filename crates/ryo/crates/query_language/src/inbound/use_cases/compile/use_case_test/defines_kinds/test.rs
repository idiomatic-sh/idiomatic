//! A word compared with the kind column of `defines`, from the compile inbound API.

use super::super::{Given, NOWHERE, compile_twice, program_with, read_function};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::compile::errors::CompileError;
use crate::outbound::catalog::fixtures::fake::CatalogFake;
use crate::outbound::syntax::fixtures::fake::SyntaxFake;
use crate::selects;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::program::types::Alias;
use ryo_values::domain::program::types::ColumnReference;
use ryo_values::domain::program::types::Expression;
use ryo_values::domain::program::types::Filter;
use ryo_values::domain::program::types::SourceReference;
use ryo_values::domain::program::types::WrittenLiteral;
use ryo_values::domain::program::types::WrittenOperand;
use ryo_values::domain::program::types::WrittenProgram;
use ryo_values::domain::program::types::WrittenRelation;
use ryo_values::domain::program::types::WrittenTest;

/// What a row gives the use case.
type Input = fn() -> Given;
/// What `compile` answers: how many steps the plan has.
type Value = usize;
type Failure = CompileError;

fn subject(given: Input) -> Result<Value, Failure> {
    compile_twice(given())
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
            program: WrittenProgram {
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

const CASES: &[Case<Input, Value, Failure>] = &[
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
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
