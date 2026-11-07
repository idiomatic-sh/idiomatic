//! Independent typed, textual and set-digest expectations for the conformance fixtures.
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use {
    crate::domain::table_texts::service::datum, crate::domain::table_texts::types::Datum,
    crate::domain::table_texts::types::Texts, ryo_values::domain::files::types::SourceView,
};
/// The source text every conformance fixture reads its names from.
pub(crate) const SOURCE: &[u8] = b"a a";
#[derive(Clone)]
pub(crate) enum Expected {
    Cell(Value),
    Text(&'static str),
    Digest(u8),
}
pub(crate) struct Fixture {
    pub(crate) operation: Operation,
    pub(crate) inputs: Vec<Table>,
    pub(crate) bindings: Vec<ParameterBinding>,
    pub(crate) symbols: Symbols,
    pub(crate) expected: Vec<Vec<Expected>>,
}
pub(super) fn cells(rows: Vec<Vec<Value>>) -> Vec<Vec<Expected>> {
    rows.into_iter()
        .map(|row| row.into_iter().map(Expected::Cell).collect())
        .collect()
}
pub(crate) fn assert_expected(
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
