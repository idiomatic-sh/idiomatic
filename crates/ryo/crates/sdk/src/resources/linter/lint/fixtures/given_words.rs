//! The test data of `linter lint` that fixes what a run of the language of words is given: its rules, each a root query over
//! one catalog that holds the fragment they share, as a pack's rules are; and the runs of the rows that give a kept store the
//! same or one thing changed.

use super::words::{WORDS, WORDS_AGAIN};
use crate::domain::file::types::RelativePath;
use crate::domain::language::types::{LanguageName, SourceLanguage};
use crate::domain::query::types::{QueryModuleCatalog, QueryModuleSource};
use crate::resources::linter::lint::types::{Corpus, LintRequest, Rule};
use crate::resources::query::RyoQlQuery;
use std::collections::BTreeMap;

/// A fragment the rules of a request share: a module no rule of the fixture imports, which every rule's query names.
const FRAGMENT: &str = "output relation<id: node, file: file, span: span>;\nlet found = select id, file, span from \"syntax-tree\" where kind = \"word\";\nreturn found;";

/// The same fragment under another text.
const FRAGMENT_AGAIN: &str = "output relation<id: node, file: file, span: span>;\nlet words = select id, file, span from \"syntax-tree\" where kind = \"word\";\nreturn words;";

/// The rule of the words alone, and with a second rule of the same query beside it.
const ONE_RULE: &[(&str, &str)] = &[("words", WORDS)];
/// The rule of the words under another text.
const ONE_RULE_AGAIN: &[(&str, &str)] = &[("words", WORDS_AGAIN)];
/// The rule of the words under another text, and a second rule beside it.
const TWO_RULES: &[(&str, &str)] = &[("words", WORDS_AGAIN), ("words-too", WORDS)];

/// What each run of the `Given` rows is given, in order: what the run before it was given, or that with one thing changed.
pub(crate) const GIVEN: &[(&str, GivenWords)] = &[
    (
        "first",
        GivenWords {
            rules: ONE_RULE,
            fragment: FRAGMENT,
            language: Reads::Recognized,
        },
    ),
    (
        "same",
        GivenWords {
            rules: ONE_RULE,
            fragment: FRAGMENT,
            language: Reads::Recognized,
        },
    ),
    (
        "rule text changed",
        GivenWords {
            rules: ONE_RULE_AGAIN,
            fragment: FRAGMENT,
            language: Reads::Recognized,
        },
    ),
    (
        "same",
        GivenWords {
            rules: ONE_RULE_AGAIN,
            fragment: FRAGMENT,
            language: Reads::Recognized,
        },
    ),
    (
        "fragment text changed",
        GivenWords {
            rules: ONE_RULE_AGAIN,
            fragment: FRAGMENT_AGAIN,
            language: Reads::Recognized,
        },
    ),
    (
        "same",
        GivenWords {
            rules: ONE_RULE_AGAIN,
            fragment: FRAGMENT_AGAIN,
            language: Reads::Recognized,
        },
    ),
    (
        "one more rule selected",
        GivenWords {
            rules: TWO_RULES,
            fragment: FRAGMENT_AGAIN,
            language: Reads::Recognized,
        },
    ),
    (
        "same",
        GivenWords {
            rules: TWO_RULES,
            fragment: FRAGMENT_AGAIN,
            language: Reads::Recognized,
        },
    ),
    (
        "the language named",
        GivenWords {
            rules: TWO_RULES,
            fragment: FRAGMENT_AGAIN,
            language: Reads::Words,
        },
    ),
    (
        "same",
        GivenWords {
            rules: TWO_RULES,
            fragment: FRAGMENT_AGAIN,
            language: Reads::Words,
        },
    ),
];

/// What a run of the fixture is given: the rules it selects, each an id and the text of its root query, the text of the one
/// fragment their queries share, and whether the request names the language of words.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GivenWords {
    /// The selected rules: each one's id and the text of its root query.
    pub(crate) rules: &'static [(&'static str, &'static str)],
    /// The text of the fragment the rules' queries share.
    pub(crate) fragment: &'static str,
    /// Which language the request says reads the files.
    pub(crate) language: Reads,
}

/// Which language a request of the fixture says reads the files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reads {
    /// The one that recognises each file.
    Recognized,
    /// The language of words, named.
    Words,
}

/// A request over the one listed file `words.walk` of `root`, given what `given` says: its rules, each over the one catalog
/// of the shared fragment, as a pack's rules are.
pub(crate) fn given_request(root: &std::path::Path, given: GivenWords) -> LintRequest {
    let fragment = QueryModuleSource::try_new("words/found", given.fragment).expect("a module");
    let catalog = QueryModuleCatalog::try_new([fragment]).expect("a catalog");
    let rules: BTreeMap<_, _> = given
        .rules
        .iter()
        .map(|(id, text)| {
            let query = RyoQlQuery::with_modules(&format!("rules/{id}"), text, &catalog)
                .expect("a query over the catalog");
            let rule = Rule {
                query,
                ..super::rules::ordinary(text)
            };
            (super::rules::rule_id(id), rule)
        })
        .collect();
    LintRequest {
        corpus: Corpus::Listed(vec![RelativePath("words.walk".into())]),
        source_language: match given.language {
            Reads::Recognized => SourceLanguage::Recognized,
            Reads::Words => SourceLanguage::Named(LanguageName("words".into())),
        },
        ..super::rules::listed(root, rules)
    }
}
