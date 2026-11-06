//! Domain behavior from typed values, independent of parsing and adapters.
use crate::cases::{Case, CaseName, Expect, Word, lowering_builder, run};
use crate::domain::lowering::errors::LoweringError;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types as parts;
use ryo_values::domain::plans::types as plans;
use ryo_values::domain::program::types as syntax;
type Input = fn();
type Value = ();
type Failure = std::convert::Infallible;

fn comparison() -> syntax::Filter {
    syntax::Filter {
        left: syntax::WrittenOperand::Column(syntax::ColumnReference {
            alias: syntax::Alias::Unwritten,
            column: syntax::ColumnName("name".into()),
        }),
        test: syntax::WrittenTest::Equal,
        right: syntax::WrittenOperand::Text(syntax::WrittenLiteral("main".into())),
        span: lowering_builder().span,
    }
}
fn typed_comparison_keeps_operands_and_test() {
    let mut builder = lowering_builder();
    let fields = crate::domain::resolving::service::relation_fields(Relation::Function);
    let comparison = super::service::comparison_of(
        &mut builder,
        &comparison(),
        crate::domain::lowering::types::Addressable {
            fields: &fields,
            sides: &[],
        },
    )
    .unwrap();
    assert_eq!(
        comparison,
        parts::Comparison {
            test: parts::Test::Equal,
            left: parts::Operand::Column(parts::ColumnIndex(1)),
            right: parts::Operand::Literal(parts::QueryLiteral::Text(parts::LiteralText(
                "main".into()
            ))),
            origin: parts::ComparisonOrigin::Unrecorded
        }
    );
}
fn incompatible_operand_keeps_exact_location() {
    let mut builder = lowering_builder();
    let fields = crate::domain::resolving::service::relation_fields(Relation::Function);
    let mut comparison = comparison();
    comparison.left = syntax::WrittenOperand::Column(syntax::ColumnReference {
        alias: syntax::Alias::Unwritten,
        column: syntax::ColumnName("id".into()),
    });
    assert_eq!(
        super::service::comparison_of(
            &mut builder,
            &comparison,
            crate::domain::lowering::types::Addressable {
                fields: &fields,
                sides: &[]
            }
        ),
        Err(LoweringError::KindMismatch {
            span: comparison.span
        })
    );
}

/// The refusal a filter meets over the columns of one relation.
fn refusal_of(relation: Relation, filter: &syntax::Filter) -> LoweringError {
    let mut builder = lowering_builder();
    let fields = crate::domain::resolving::service::relation_fields(relation);
    super::service::comparison_of(
        &mut builder,
        filter,
        crate::domain::lowering::types::Addressable {
            fields: &fields,
            sides: &[],
        },
    )
    .expect_err("the filter is refused")
}
fn column(name: &str) -> syntax::WrittenOperand {
    syntax::WrittenOperand::Column(syntax::ColumnReference {
        alias: syntax::Alias::Unwritten,
        column: syntax::ColumnName(name.into()),
    })
}
fn written(value: &str) -> syntax::WrittenOperand {
    syntax::WrittenOperand::Text(syntax::WrittenLiteral(value.into()))
}
fn a_function_given_the_wrong_number_of_arguments_is_refused() {
    let mut filter = comparison();
    filter.left = syntax::WrittenOperand::Function(syntax::WrittenApplied {
        function: syntax::WrittenFunction::Pascal,
        arguments: vec![column("name"), column("name")],
        span: filter.span,
    });
    assert!(matches!(
        refusal_of(Relation::Function, &filter),
        LoweringError::FunctionArity { .. }
    ));
}
fn a_function_given_a_column_that_is_not_text_is_refused() {
    let mut filter = comparison();
    filter.left = syntax::WrittenOperand::Function(syntax::WrittenApplied {
        function: syntax::WrittenFunction::Pascal,
        arguments: vec![column("id")],
        span: filter.span,
    });
    assert!(matches!(
        refusal_of(Relation::Function, &filter),
        LoweringError::NotText { .. }
    ));
}
fn an_affix_test_of_a_column_that_is_not_text_is_refused() {
    let mut filter = comparison();
    filter.left = column("id");
    filter.test = syntax::WrittenTest::StartsWith;
    assert!(matches!(
        refusal_of(Relation::Function, &filter),
        LoweringError::NotText { .. }
    ));
}
fn a_literal_pattern_that_is_not_a_regex_is_refused() {
    let mut filter = comparison();
    filter.test = syntax::WrittenTest::Matches;
    filter.right = written("[");
    assert!(matches!(
        refusal_of(Relation::Function, &filter),
        LoweringError::InvalidRegex { .. }
    ));
}
fn a_word_the_column_does_not_hold_is_refused() {
    let mut filter = comparison();
    filter.left = column("kind");
    filter.right = written("headline");
    assert!(matches!(
        refusal_of(Relation::Defines, &filter),
        LoweringError::UnknownWord { .. }
    ));
}

fn more_origins_than_the_id_space_holds_are_refused() {
    assert!(matches!(
        super::service::comparison_origin_id(
            crate::domain::lowering::types::OriginCount(usize::MAX),
            lowering_builder().span
        ),
        Err(LoweringError::TooManyComparisonOrigins { .. })
    ));
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("typed_comparison_keeps_operands_and_test"),
        word: Word::Succeeds,
        input: || typed_comparison_keeps_operands_and_test,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("incompatible_operand_keeps_exact_location"),
        word: Word::Succeeds,
        input: || incompatible_operand_keeps_exact_location,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_function_given_the_wrong_number_of_arguments_is_refused"),
        word: Word::Succeeds,
        input: || a_function_given_the_wrong_number_of_arguments_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_function_given_a_column_that_is_not_text_is_refused"),
        word: Word::Succeeds,
        input: || a_function_given_a_column_that_is_not_text_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_affix_test_of_a_column_that_is_not_text_is_refused"),
        word: Word::Succeeds,
        input: || an_affix_test_of_a_column_that_is_not_text_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_literal_pattern_that_is_not_a_regex_is_refused"),
        word: Word::Succeeds,
        input: || a_literal_pattern_that_is_not_a_regex_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_word_the_column_does_not_hold_is_refused"),
        word: Word::Succeeds,
        input: || a_word_the_column_does_not_hold_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("more_origins_than_the_id_space_holds_are_refused"),
        word: Word::Succeeds,
        input: || more_origins_than_the_id_space_holds_are_refused,
        expect: Expect::Value(()),
    },
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
