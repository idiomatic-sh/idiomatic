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
            origin: None
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
];

fn subject(check: Input) -> Result<Value, Failure> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
