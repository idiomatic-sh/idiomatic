//! A row is the texts under the keys, in key order; a missing key refuses.
use super::service::rows;
use super::types::{RowTexts, TableRowOutput};
use crate::domain::contract::types::{PortableOperatorFault, SemanticInterrupted};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::TableKey;
use ryo_values::domain::query_bindings::types::{BindingCell, BindingTable};
use ryo_values::domain::plan_parts::types::LiteralText;
struct Collect(Vec<String>);
impl TableRowOutput for Collect {
    fn row(&mut self, texts: RowTexts<'_>) -> Result<(), SemanticInterrupted> {
        self.0.push(
            texts
                .0
                .iter()
                .map(|text| text.0)
                .collect::<Vec<_>>()
                .join("|"),
        );
        Ok(())
    }
}
type Input = &'static [&'static str];
type Value = std::borrow::Cow<'static, str>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("keys_order_the_columns"),
        word: Word::Succeeds,
        input: || &["b", "a"],
        expect: Expect::Value(std::borrow::Cow::Borrowed("2|1")),
    },
    Case {
        name: CaseName("a_missing_key_is_unbound"),
        word: Word::Refuses,
        input: || &["a", "z"],
        expect: Expect::Error(PortableOperatorFault::UnboundKey),
    },
];
fn subject(keys: Input) -> Result<Value, Failure> {
    let cell = |key: &str, value: &str| BindingCell {
        key: LiteralText(key.into()),
        value: LiteralText(value.into()),
    };
    let table = BindingTable(vec![cell("a", "1"), cell("b", "2")]);
    let keys: Vec<TableKey> = keys.iter().map(|key| TableKey((*key).into())).collect();
    let mut out = Collect(Vec::new());
    rows(&[table], &keys, &mut out)?;
    Ok(std::borrow::Cow::Owned(out.0.join(",")))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
