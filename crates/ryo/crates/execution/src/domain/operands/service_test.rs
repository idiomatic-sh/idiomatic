//! The `operands` service's cases, on values alone.

use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Integer;

use super::service::resolve;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::functions::types::MadeText;
use crate::domain::operating::errors::OperatingError;
use {crate::domain::table_texts::types::Datum, ryo_values::domain::files::types::SourceView, crate::domain::table_texts::types::Texts};
use ryo_values::domain::plan_parts::types::{Operand, ParameterId, QueryLiteral, WholeNumber};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Resolves a number the query wrote.
    Literal,
    /// Resolves a parameter nothing is bound to.
    Unbound,
}

/// What a row of the cases does.
type Input = Scenario;
/// Whether the operand is what the scenario expects.
type Value = usize;
type Failure = OperatingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    let symbols = Symbols::default();
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let mut made = MadeText::default();
    match scenario {
        Scenario::Literal => {
            let operand = Operand::Literal(QueryLiteral::Number(Integer(7)));
            let datum = resolve(&operand, &[], &[], &texts, &mut made)?;
            Ok(usize::from(datum == Datum::Number(WholeNumber(7))))
        }
        Scenario::Unbound => {
            let operand = Operand::Parameter(ParameterId(0));
            resolve(&operand, &[], &[], &texts, &mut made).map(|_| 0)
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_number_the_query_wrote_is_that_number"),
        word: Word::Succeeds,
        input: || Scenario::Literal,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_parameter_nothing_is_bound_to_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Unbound,
        expect: Expect::Error(OperatingError::Unbound),
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
