//! The refusal of an unknown kind, made before any file is read: the plan's words for a kind (the shared kind check of the query
//! language) against the kinds the linked languages have.

use super::refusal::Refusal;
use ryo_query_language::domain::kind_check::service::{asked, names_kind};
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, KindWord, LanguageName};
use ryo_values::domain::finish_plans::types::FinishPlan;

/// The refusal of the first word `plan` writes for a kind that none of `languages` (each with the names of its kinds) has, or nothing.
#[must_use]
pub(crate) fn unknown(plan: &FinishPlan, languages: impl FnOnce() -> Vec<(String, Vec<String>)>) -> Option<Refusal> {
    let asked = asked(plan);
    if asked.is_empty() {
        return None;
    }
    let languages = languages();
    let vocabularies = KindVocabularies(
        languages
            .iter()
            .map(|(name, kinds)| KindVocabulary {
                language: LanguageName(name.as_str().into()),
                kinds: kinds.iter().map(|kind| KindWord(kind.as_str().into())).collect(),
                fields: Vec::new(),
                fixed: Vec::new(),
            })
            .collect(),
    );
    asked
        .into_iter()
        .find(|(test, word)| !names_kind(*test, word, &vocabularies))
        .map(|(_, word)| Refusal::UnknownKind { word, languages: languages.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>().join(", ") })
}
