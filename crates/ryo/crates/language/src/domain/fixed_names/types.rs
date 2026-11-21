//! The types of the `fixed_names` concept: the texts a language's fillers write by handle as the language states them, the handle
//! of one, and the names once they are checked.

use ryo_relations::domain::terms::types::NameText;

/// A place in a language's fixed names (`WalkVocabulary::fixed`): the handle of that name's word for the whole run, read with
/// `facts::service::fixed`.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::facts::service::{base_words, file_words, fixed, text};
/// use ryo_language::domain::fixed_names::service::checked;
/// use ryo_language::domain::fixed_names::types::FixedWord;
/// use ryo_relations::domain::terms::types::NameText;
///
/// let names = [NameText("next-token")];
/// let words = file_words(&base_words(checked(&names).expect("one name, stated once"), &[]));
/// assert_eq!(text(&words, fixed(&words, FixedWord(0))), NameText("next-token"));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedWord(pub u32);

/// The texts a language's fillers write by handle, as the language states them: a `FixedWord` is a place here. They become base
/// words of a run once `fixed_names::service::checked` has passed them: each stated once and none empty.
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fixed_names::types::FixedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// assert_eq!(FixedNames(vec![NameText("next-token")]).0.len(), 1);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FixedNames(pub Vec<NameText<'static>>);

/// A language's fixed names as `fixed_names::service::checked` passed them: each stated once and none empty, so the place of a
/// name is its handle and its word is that place after the empty text. It borrows the names, from a language's own list or from
/// texts a run holds, and is made by `checked` alone; the default holds no name.
///
/// ```compile_fail,E0603
/// let forged = ryo_language::domain::fixed_names::types::CheckedNames(&[]);
/// ```
///
/// # Examples
///
/// ```
/// use ryo_language::domain::fixed_names::service::checked;
/// use ryo_language::domain::fixed_names::types::CheckedNames;
/// use ryo_relations::domain::terms::types::NameText;
///
/// assert_eq!(checked(&[]), Ok(CheckedNames::default()));
/// assert_ne!(checked(&[NameText("next-token")]), Ok(CheckedNames::default()));
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CheckedNames<'n>(pub(crate) &'n [NameText<'n>]);
