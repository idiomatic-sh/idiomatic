//! Unicode word splitting with the original case-boundary and digit policy.
pub(crate) fn split(
    text: &str,
    word: &mut String,
    each: &mut dyn crate::domain::words::types::WordOutput,
) {
    word.clear();
    let mut before: Option<char> = None;
    for character in text.chars() {
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
