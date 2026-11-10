//! The `table_texts` service: the texts the derived builds read, through the derived crate's text port: a name's or a
//! file's text, and nothing for any other value.
use crate::domain::table_texts::service::datum;
use crate::domain::table_texts::types::{Datum, Texts};

/// The text of a value as the derived builds read it: a name's or a file's text, and nothing for any other value.
impl ryo_derived::domain::row_cells::types::ValueTexts for Texts<'_> {
    fn text(
        &self,
        value: ryo_relations::domain::relations::types::Value,
    ) -> Option<ryo_values::domain::row_texts::types::CellText<'_>> {
        match datum(value, self) {
            Datum::Text(text) => Some(ryo_values::domain::row_texts::types::CellText(text.0)),
            Datum::Unlocated
            | Datum::Number(_)
            | Datum::Flag(_)
            | Datum::Numbers(_)
            | Datum::Texts(_) => None,
        }
    }
}
