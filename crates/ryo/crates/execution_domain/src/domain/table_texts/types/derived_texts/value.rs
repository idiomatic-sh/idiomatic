//! The `DerivedTexts` value: the texts of a file's rows, read through the derived builds' port.

use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};
use ryo_derived::domain::row_cells::types::ValueTexts;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::row_texts::types::CellText;

/// The texts of a file's rows as the derived builds read them.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::table_texts::types::derived_texts::value::DerivedTexts;
/// fn named(_: Option<&DerivedTexts<'_>>) {}
/// ```
#[derive(Clone, Copy, Debug)]
pub struct DerivedTexts<'a>(pub Texts<'a>);

/// The text of a value as the derived builds read it: a name's or a file's text, and nothing for any other value.
impl ValueTexts for DerivedTexts<'_> {
    fn text(&self, value: Value) -> Option<CellText<'_>> {
        match datum(value, &self.0) {
            Datum::Text(text) => Some(CellText(text.0)),
            Datum::Unlocated
            | Datum::Number(_)
            | Datum::Flag(_)
            | Datum::Numbers(_)
            | Datum::Texts(_) => None,
        }
    }
}
