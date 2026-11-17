//! The `operands` service's cases, on values alone.

use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Integer;

use super::service::resolve;
use crate::domain::functions::types::MadeText;
use crate::domain::operands::errors::OperandsError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{Operand, ParameterId, QueryLiteral, WholeNumber};
use {
    crate::domain::table_texts::types::Datum, crate::domain::table_texts::types::Texts,
    ryo_values::domain::files::types::SourceView,
};

/// What an operand resolves to: a whole number, or any other datum as it prints.
#[derive(Debug, PartialEq)]
enum Resolved {
    Number(WholeNumber),
    Other(String),
}

/// The operand resolved over no row and no text, with parameter 0 bound to sets of text when it says so.
type Input = (Operand, Bound);

/// What parameter 0 is bound to.
#[derive(Clone, Copy)]
enum Bound {
    Nothing,
    TextSets,
}
type Value = Resolved;
type Failure = OperandsError;

fn subject((operand, bound): Input) -> Result<Value, Failure> {
    let bindings = match bound {
        Bound::Nothing => Vec::new(),
        Bound::TextSets => {
            vec![ryo_values::domain::query_bindings::types::ParameterBinding::TextSets(Vec::new())]
        }
    };
    let symbols = Symbols::default();
    let texts = Texts {
        symbols: &symbols,
        source: SourceView(&[]),
        paths: &[],
        arenas: &[],
    };
    let mut made = MadeText::default();
    Ok(
        match resolve(&operand, &[], &bindings, &texts, &mut made)? {
            Datum::Number(number) => Resolved::Number(number),
            other @ (Datum::Unlocated
            | Datum::Flag(_)
            | Datum::Text(_)
            | Datum::Numbers(_)
            | Datum::Texts(_)) => Resolved::Other(format!("{other:?}")),
        },
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_number_the_query_wrote_is_that_number"),
        word: Word::Succeeds,
        input: || {
            (
                Operand::Literal(QueryLiteral::Number(Integer(7))),
                Bound::Nothing,
            )
        },
        expect: Expect::Value(Resolved::Number(WholeNumber(7))),
    },
    Case {
        name: CaseName("a_parameter_nothing_is_bound_to_is_refused"),
        word: Word::Refuses,
        input: || (Operand::Parameter(ParameterId(0)), Bound::Nothing),
        expect: Expect::Error(OperandsError::Unbound),
    },
    Case {
        name: CaseName("a_parameter_bound_to_sets_of_text_is_refused"),
        word: Word::Refuses,
        input: || (Operand::Parameter(ParameterId(0)), Bound::TextSets),
        expect: Expect::Error(OperandsError::WrongBinding),
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
