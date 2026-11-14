//! Unicode word splitting with the original case-boundary and digit policy.
use crate::domain::words::types::WordScratch;
use ryo_values::domain::row_texts::types::CellText;
pub(crate) fn split(
    text: CellText<'_>,
    scratch: WordScratch<'_>,
    each: &mut dyn crate::domain::words::types::WordOutput,
) {
    let word = scratch.0;
    word.clear();
    let mut before: Option<char> = None;
    for character in text.0.chars() {
        if !character.is_alphanumeric() {
            if !word.is_empty() {
                each.word(CellText(word.as_str()));
                word.clear();
            }
            before = None;
            continue;
        }
        if character.is_uppercase()
            && before.is_some_and(|previous| previous.is_lowercase() || previous.is_ascii_digit())
            && !word.is_empty()
        {
            each.word(CellText(word.as_str()));
            word.clear();
        }
        word.extend(character.to_lowercase());
        before = Some(character);
    }
    if !word.is_empty() {
        each.word(CellText(word.as_str()));
        word.clear();
    }
}

/// Splits the text of a scheduled row's column into words and emits the row once for each word, with the
/// word appended; a cell that is not text has no row.
pub(crate) fn evaluate(
    row: &[ryo_relations::domain::relations::types::Value],
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    scratch: &mut crate::domain::contract::types::RowScratch,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault> {
    let value = ryo_values::domain::rows::service::column_in(row, column)
        .copied()
        .ok_or(
            crate::domain::contract::types::PortableOperatorFault::NoSuchColumn { index: column },
        )?;
    let crate::domain::contract::types::RowScratch {
        row: joined,
        captured,
        text,
        word,
        ..
    } = scratch;
    text.0.clear();
    if driver.text(value, text) == crate::domain::contract::types::TextFound::NotText {
        return Ok(());
    }
    captured.clear();
    split(
        CellText(text.0.as_str()),
        WordScratch(&mut word.0),
        &mut crate::domain::words::types::WordMinting {
            driver: &mut *driver,
            captured: &mut *captured,
        },
    );
    for each in captured.iter() {
        joined.clear();
        joined.extend_from_slice(row);
        joined.push(*each);
        driver.emit(joined)?;
    }
    Ok(())
}

/// The one lowering of a word split: its text column, for the `text_map` kernel.
pub(crate) fn lower(
    text: ryo_values::domain::plan_parts::types::ColumnIndex,
) -> super::types::WordsParts {
    super::types::WordsParts { text }
}
