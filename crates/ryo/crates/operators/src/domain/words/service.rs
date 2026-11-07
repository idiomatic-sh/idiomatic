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

/// Interns each word as derived text.
struct WordMinting<'a> {
    driver: &'a mut dyn crate::domain::evaluation::types::InvocationDriver,
    captured: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
}

impl crate::domain::words::types::WordOutput for WordMinting<'_> {
    fn word(&mut self, text: CellText<'_>) {
        let made = self.driver.mint(text);
        self.captured.push(made);
    }
}

/// Splits the text of a scheduled row's column into words and emits the row once for each word, with the
/// word appended; a cell that is not text has no row.
pub(crate) fn evaluate(
    row: &[ryo_relations::domain::relations::types::Value],
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
    driver: &mut dyn crate::domain::evaluation::types::InvocationDriver,
    scratch: &mut crate::domain::evaluation::types::EvaluationScratch,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault> {
    let value = crate::domain::evaluation::service::cell(row, column)?;
    let crate::domain::evaluation::types::EvaluationScratch {
        row: joined,
        captured,
        text,
        word,
        ..
    } = scratch;
    text.clear();
    if driver.text(value, text) == crate::domain::evaluation::types::TextFound::NotText {
        return Ok(());
    }
    captured.clear();
    split(
        CellText(text.as_str()),
        WordScratch(word),
        &mut WordMinting {
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
