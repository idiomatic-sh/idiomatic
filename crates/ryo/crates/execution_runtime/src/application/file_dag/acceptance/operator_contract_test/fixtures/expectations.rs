//! Independent typed, textual and set-digest expectations for the conformance fixtures.
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use {
    ryo_execution_domain::domain::table_texts::service::datum, ryo_execution_domain::domain::table_texts::types::Datum,
    ryo_execution_domain::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
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
/// Asserts an answer's rows against the expected rows, cell by cell.
pub fn assert_expected(
    expected: &[Vec<Expected>],
    actual: &[Vec<Value>],
    symbols: &Symbols,
) {
    assert_eq!(expected.len(), actual.len());
    let mut digests = std::collections::BTreeMap::new();
    let texts = Texts {
        symbols,
        source: SourceView(SOURCE),
        paths: &[],
        arenas: &[],
    };
    for (expected, actual) in expected.iter().zip(actual) {
        assert_eq!(expected.len(), actual.len());
        for (expected, actual) in expected.iter().zip(actual) {
            match expected {
                Expected::Cell(value) => assert_eq!(value, actual),
                Expected::Text(text) => {
                    let Datum::Text(value) = datum(*actual, &texts) else {
                        panic!("expected text")
                    };
                    assert_eq!(*text, value.0);
                }
                Expected::Digest(group) => {
                    assert!(matches!(actual, Value::Number(_)));
                    if let Some(previous) = digests.insert(group, actual) {
                        assert_eq!(previous, actual);
                    }
                }
            }
        }
    }
}

use ryo_values::domain::rows::types::Table;
