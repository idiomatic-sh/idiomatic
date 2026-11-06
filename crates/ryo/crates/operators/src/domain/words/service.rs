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
                each.word(word.as_str());
                word.clear();
            }
            before = None;
            continue;
        }
        if character.is_uppercase()
            && before.is_some_and(|previous| previous.is_lowercase() || previous.is_ascii_digit())
            && !word.is_empty()
        {
            each.word(word.as_str());
            word.clear();
        }
        word.extend(character.to_lowercase());
        before = Some(character);
    }
    if !word.is_empty() {
        each.word(word.as_str());
        word.clear();
    }
}
