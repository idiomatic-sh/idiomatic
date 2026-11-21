//! The `fixed_names` service: the one check a language's fixed names pass before they become base words of a run.

use super::errors::FixedNamesError;
use super::types::{CheckedNames, FixedWord};
use ryo_relations::domain::terms::types::NameText;

/// The names, checked: each stated once and none empty, so the place of a name is its handle by construction. Called once for
/// a language in a run, where its names become base words (`facts::service::base_words`), with the language's own list
/// (`WalkVocabulary::fixed`) or with texts the run holds; the names are borrowed, never copied.
///
/// # Errors
///
/// `Empty` when a name is the empty text, which is word 0 of every run; `Twice` when a name is stated at two places: the
/// second would have no word of its own. The first such name in the order given is the one refused.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fixed_names::errors::FixedNamesError;
/// use ryo_language::domain::fixed_names::service::checked;
/// use ryo_language::domain::fixed_names::types::FixedWord;
/// use ryo_relations::domain::terms::types::NameText;
///
/// // A language's own names, and texts a run holds: both are borrowed.
/// assert!(checked(&[NameText("next-token"), NameText("matching-delimiter")]).is_ok());
/// let held: Vec<Box<str>> = vec!["next-token".into(), "next-token".into()];
/// let names: Vec<NameText<'_>> = held.iter().map(|name| NameText(name)).collect();
/// assert_eq!(checked(&names), Err(FixedNamesError::Twice { first: FixedWord(0), again: FixedWord(1) }));
/// ```
pub fn checked<'n>(names: &'n [NameText<'n>]) -> Result<CheckedNames<'n>, FixedNamesError> {
    let mut places: hashbrown::HashMap<NameText<'n>, FixedWord> =
        hashbrown::HashMap::with_capacity(names.len());
    for (number, name) in names.iter().enumerate() {
        let place = FixedWord(u32::try_from(number).unwrap_or(u32::MAX));
        if name.0.is_empty() {
            return Err(FixedNamesError::Empty { place });
        }
        if let Some(first) = places.insert(*name, place) {
            return Err(FixedNamesError::Twice {
                first,
                again: place,
            });
        }
    }
    Ok(CheckedNames(names))
}
