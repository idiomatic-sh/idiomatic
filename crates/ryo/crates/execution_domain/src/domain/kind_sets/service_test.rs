//! The `kind_sets` service's cases: which comparisons of a select name a kind, on values alone.

use super::service::names_a_kind;
use crate::domain::operating::types::KindNaming;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};

/// What a row of the cases asks: the test, and the column the left side reads.
type Input = (Test, u32);
/// Whether the comparison names a kind.
type Value = KindNaming;
type Failure = ();

fn comparison((test, column): Input) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("function".into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(names_a_kind(&comparison(input)))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_equality_of_the_kind_column_with_a_text_names_a_kind"),
        word: Word::Succeeds,
        input: || (Test::Equal, 1),
        expect: Expect::Value(KindNaming::Names),
    },
    Case {
        name: CaseName("a_glob_of_the_kind_column_names_kinds"),
        word: Word::Succeeds,
        input: || (Test::GlobAny, 1),
        expect: Expect::Value(KindNaming::Names),
    },
    Case {
        name: CaseName("an_equality_of_another_column_names_no_kind"),
        word: Word::Succeeds,
        input: || (Test::Equal, 2),
        expect: Expect::Value(KindNaming::Silent),
    },
    Case {
        name: CaseName("an_ordering_of_the_kind_column_names_no_kind"),
        word: Word::Succeeds,
        input: || (Test::LessThan, 1),
        expect: Expect::Value(KindNaming::Silent),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
