//! Independent typed, textual and set-digest expectations for the conformance fixtures.
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use {
    ryo_operators::domain::table_texts::service::datum,
    ryo_operators::domain::table_texts::types::Datum,
    ryo_operators::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
};
/// The source text every conformance fixture reads its names from.
pub const SOURCE: &[u8] = b"a a";
/// One expected cell of an operation's answer.
#[derive(Clone, Copy, Debug)]
pub enum Expected {
    /// The cell's value.
    Cell(Value),
    /// A text cell, compared by its text.
    Text(&'static str),
    /// A digest cell, compared by the byte its fixture digest repeats.
    Digest(u8),
}
/// One operation with its authored inputs and its expected answer.
#[derive(Debug)]
pub struct Fixture {
    /// The operation under proof.
    pub operation: Operation,
    /// Its input tables, in input order.
    pub inputs: Vec<Table>,
    /// The parameter bindings the operation reads.
    pub bindings: Vec<ParameterBinding>,
    /// The symbols the inputs' texts are interned in.
    pub symbols: Symbols,
    /// The expected rows of the answer.
    pub expected: Vec<Vec<Expected>>,
}
pub(super) fn cells(rows: Vec<Vec<Value>>) -> Vec<Vec<Expected>> {
    rows.into_iter()
        .map(|row| row.into_iter().map(Expected::Cell).collect())
        .collect()
}
/// How an answer's rows compare with the expected rows, cell by cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Conformance {
    /// Every row and every cell is the one expected of it.
    Conforms,
    /// The answer has another number of rows: the expected and the answered count.
    Rows(usize, usize),
    /// The answered row at this index has another number of cells than expected.
    Width(usize),
    /// The answered cell at this row and column differs from the one expected of it.
    Cell(usize, usize),
}

/// Compares an answer's rows with the expected rows, cell by cell: a text cell by its text, and a digest cell as a
/// number equal to every other cell of its digest group.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::{
///     Conformance, Expected, conformance,
/// };
/// use ryo_language::domain::symbols::types::Symbols;
/// use ryo_relations::domain::relations::types::{Integer, Value};
///
/// let one = Value::Number(Integer(1));
/// let symbols = Symbols::default();
/// assert_eq!(conformance(&[vec![Expected::Cell(one)]], &[vec![one]], &symbols), Conformance::Conforms);
/// assert_eq!(conformance(&[], &[vec![one]], &symbols), Conformance::Rows(0, 1));
/// ```
pub fn conformance(
    expected: &[Vec<Expected>],
    actual: &[Vec<Value>],
    symbols: &Symbols,
) -> Conformance {
    if expected.len() != actual.len() {
        return Conformance::Rows(expected.len(), actual.len());
    }
    let mut digests = std::collections::BTreeMap::new();
    let texts = Texts {
        symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
    };
    for (row, (expected, actual)) in expected.iter().zip(actual).enumerate() {
        if expected.len() != actual.len() {
            return Conformance::Width(row);
        }
        for (column, (expected, actual)) in expected.iter().zip(actual).enumerate() {
            let same = match expected {
                Expected::Cell(value) => value == actual,
                Expected::Text(text) => {
                    matches!(datum(*actual, &texts), Datum::Text(value) if value.0 == *text)
                }
                Expected::Digest(group) => {
                    matches!(actual, Value::Number(_))
                        && *digests.entry(group).or_insert(actual) == actual
                }
            };
            if !same {
                return Conformance::Cell(row, column);
            }
        }
    }
    Conformance::Conforms
}

use ryo_values::domain::rows::types::Table;
