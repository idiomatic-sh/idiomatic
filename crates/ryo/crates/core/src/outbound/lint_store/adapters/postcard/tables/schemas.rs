//! The tables document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use super::super::files::schemas::{FileSchema, file_from, file_schema};
use super::super::findings::schemas::{
    FindingSchema, KeySchema, ValueSchema, findings_from, findings_schema,
};
use super::super::strings::schemas::{StringsSchema, TextSchema, file_number, named, text};
use super::super::unknowns::schemas::{UnknownSchema, unknown_from, unknown_schema};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::{EntryObservation, ObservedEntryState};
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::runs::constants::SYNTAX_REJECTED;
use ryo_values::domain::runs::types::{ManifestRefused, RuleId, SkipReason, Skipped};
use ryo_values::domain::stored_facts::types::{Digest, ReadFailure};
use ryo_values::domain::stored_findings::types::StoredUnread;
use ryo_values::domain::stores::types::{
    Crossing, Stored, StoredFile, StoredFrontier, StoredHanded, StoredRule,
};
use std::borrow::Cow;

/// One exact project-entry observation retained for warm context validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct EntrySchema {
    pub(in crate::outbound::lint_store::adapters::postcard) path: TextSchema,
    pub(in crate::outbound::lint_store::adapters::postcard) state: EntryStateSchema,
}

/// Stable entry-kind tags in the checked store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) enum EntryStateSchema {
    File,
    Directory,
    Other,
    Missing,
    Unknown,
}

impl From<ObservedEntryState> for EntryStateSchema {
    fn from(state: ObservedEntryState) -> Self {
        match state {
            ObservedEntryState::File => Self::File,
            ObservedEntryState::Directory => Self::Directory,
            ObservedEntryState::Other => Self::Other,
            ObservedEntryState::Missing => Self::Missing,
            ObservedEntryState::Unknown => Self::Unknown,
        }
    }
}

impl From<EntryStateSchema> for ObservedEntryState {
    fn from(state: EntryStateSchema) -> Self {
        match state {
            EntryStateSchema::File => Self::File,
            EntryStateSchema::Directory => Self::Directory,
            EntryStateSchema::Other => Self::Other,
            EntryStateSchema::Missing => Self::Missing,
            EntryStateSchema::Unknown => Self::Unknown,
        }
    }
}

/// A rule: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct RuleSchema {
    /// Its id.
    pub(in crate::outbound::lint_store::adapters::postcard) rule: TextSchema,
    /// What its query, parameters and files came to.
    pub(in crate::outbound::lint_store::adapters::postcard) digest: [u8; 16],
    /// Whether it reads across files.
    pub(in crate::outbound::lint_store::adapters::postcard) across: bool,
}

/// What a file hands one rule that reads across files: one row for each file and rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct FrontierSchema {
    /// The file, by its place among the files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    /// The rule, by its place among the rules.
    pub(in crate::outbound::lint_store::adapters::postcard) rule: u32,
    /// What the file hands it.
    pub(in crate::outbound::lint_store::adapters::postcard) digest: [u8; 16],
}

/// A file no language read: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct UnreadSchema {
    /// The file, by its place among the files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    /// What the language said when it failed; none when no language reads such a file.
    pub(in crate::outbound::lint_store::adapters::postcard) failure: Option<TextSchema>,
}

/// A manifest a language's tools refused, or a rule skipped on a file: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct TripleSchema {
    /// The language, or the rule.
    pub(in crate::outbound::lint_store::adapters::postcard) first: TextSchema,
    /// The manifest, or the file whose relation is read in part.
    pub(in crate::outbound::lint_store::adapters::postcard) path: TextSchema,
    /// What the tool said, or what is partial there and why.
    pub(in crate::outbound::lint_store::adapters::postcard) said: TextSchema,
}

/// The small tables of the store: what a warm run reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct TablesSchema<'a> {
    /// Every text the rows name, each once.
    #[serde(borrow)]
    pub(in crate::outbound::lint_store::adapters::postcard) strings: Vec<Cow<'a, str>>,
    /// The rules.
    pub(in crate::outbound::lint_store::adapters::postcard) rules: Vec<RuleSchema>,
    /// The files.
    pub(in crate::outbound::lint_store::adapters::postcard) files: Vec<FileSchema>,
    /// What each file hands each rule that reads across files.
    pub(in crate::outbound::lint_store::adapters::postcard) frontiers: Vec<FrontierSchema>,
    /// The findings.
    pub(in crate::outbound::lint_store::adapters::postcard) findings: Vec<FindingSchema>,
    /// The values of the findings' keys.
    pub(in crate::outbound::lint_store::adapters::postcard) finding_keys: Vec<KeySchema>,
    /// The findings' columns.
    pub(in crate::outbound::lint_store::adapters::postcard) finding_values: Vec<ValueSchema>,
    /// The files no language read.
    pub(in crate::outbound::lint_store::adapters::postcard) unread: Vec<UnreadSchema>,
    /// The manifests the tools refused.
    pub(in crate::outbound::lint_store::adapters::postcard) refused: Vec<TripleSchema>,
    /// What the rules could not decide.
    pub(in crate::outbound::lint_store::adapters::postcard) unknowns: Vec<UnknownSchema>,
    /// The files the project's lookup read beyond the manifests, as the files are kept.
    pub(in crate::outbound::lint_store::adapters::postcard) consulted: Vec<FileSchema>,
    /// The files the lookup asked for and found none of, as texts.
    pub(in crate::outbound::lint_store::adapters::postcard) consulted_absent: Vec<TextSchema>,
    /// Candidate entries used for project membership, in request order.
    pub(in crate::outbound::lint_store::adapters::postcard) project_observations: Vec<EntrySchema>,
    /// Rules explicitly skipped because their required syntax was rejected.
    #[serde(default)]
    pub(in crate::outbound::lint_store::adapters::postcard) skipped: Vec<TripleSchema>,
}

impl<'a> From<&'a Stored> for TablesSchema<'a> {
    fn from(stored: &'a Stored) -> Self {
        let mut strings = StringsSchema::default();
        let rules = stored
            .rules
            .iter()
            .map(|each| RuleSchema {
                rule: text(&mut strings, &each.rule.0),
                digest: each.digest.0,
                across: each.crossing == Crossing::Across,
            })
            .collect();
        let files = stored
            .files
            .iter()
            .map(|each| file_schema(&mut strings, each))
            .collect();
        let consulted = stored
            .consulted
            .iter()
            .map(|each| file_schema(&mut strings, each))
            .collect();
        let consulted_absent = stored
            .consulted_absent
            .iter()
            .map(|each| text(&mut strings, &each.0))
            .collect();
        let project_observations = stored
            .project_observations
            .iter()
            .map(|each| EntrySchema {
                path: text(&mut strings, &each.path.0),
                state: each.state.into(),
            })
            .collect();
        let (findings, finding_keys, finding_values) =
            findings_schema(&mut strings, &stored.findings);
        let unread = stored
            .unread
            .iter()
            .map(|each| UnreadSchema {
                file: each.file.0,
                failure: match &each.reason {
                    Unreadable::NoLanguage => None,
                    Unreadable::Failed(failure) => Some(text(&mut strings, &failure.0)),
                },
            })
            .collect();
        let refused = stored
            .refused
            .iter()
            .map(|each| TripleSchema {
                first: text(&mut strings, &each.language.0),
                path: text(&mut strings, &each.path.0),
                said: text(&mut strings, &each.reason.0),
            })
            .collect();
        let unknowns = stored
            .unknowns
            .iter()
            .map(|each| unknown_schema(&mut strings, each))
            .collect();
        let skipped = stored
            .skipped
            .iter()
            .map(|each| TripleSchema {
                first: text(&mut strings, &each.rule.0),
                path: text(&mut strings, &each.path.0),
                said: text(
                    &mut strings,
                    match each.reason {
                        SkipReason::SyntaxRejected => SYNTAX_REJECTED,
                    },
                ),
            })
            .collect();
        Self {
            frontiers: stored
                .frontiers
                .iter()
                .map(|each| FrontierSchema {
                    file: each.file.0,
                    rule: each.rule.0,
                    digest: each.digest.0,
                })
                .collect(),
            strings: strings.texts,
            rules,
            files,
            findings,
            finding_keys,
            finding_values,
            unread,
            refused,
            unknowns,
            consulted,
            consulted_absent,
            project_observations,
            skipped,
        }
    }
}

impl TryFrom<TablesSchema<'_>> for Stored {
    type Error = postcard::Error;

    fn try_from(tables: TablesSchema<'_>) -> Result<Self, Self::Error> {
        let strings = &tables.strings;
        let rules = rules_from(&tables)?;
        let files: Vec<StoredFile> = tables
            .files
            .iter()
            .map(|each| file_from(strings, each))
            .collect::<Result<_, postcard::Error>>()?;
        let consulted = tables
            .consulted
            .iter()
            .map(|each| file_from(strings, each))
            .collect::<Result<_, postcard::Error>>()?;
        let consulted_absent = tables
            .consulted_absent
            .iter()
            .map(|each| named(strings, *each).map(FilePath))
            .collect::<Result<_, postcard::Error>>()?;
        let project_observations = tables
            .project_observations
            .iter()
            .map(|each| {
                Ok(EntryObservation {
                    path: FilePath(named(strings, each.path)?),
                    state: each.state.into(),
                })
            })
            .collect::<Result<Vec<_>, postcard::Error>>()?;
        let file = |at: u32| file_number(files.len(), at);
        let frontiers = tables
            .frontiers
            .iter()
            .map(|each| {
                Ok(StoredFrontier {
                    file: file(each.file)?,
                    rule: PlacedRule(each.rule),
                    digest: Digest(each.digest),
                })
            })
            .collect::<Result<_, postcard::Error>>()?;
        let unread = tables
            .unread
            .iter()
            .map(|each| {
                Ok(StoredUnread {
                    file: file(each.file)?,
                    reason: match each.failure {
                        Some(failure) => Unreadable::Failed(ReadFailure(named(strings, failure)?)),
                        None => Unreadable::NoLanguage,
                    },
                })
            })
            .collect::<Result<_, postcard::Error>>()?;
        let refused = tables
            .refused
            .iter()
            .map(|each| {
                Ok(ManifestRefused {
                    language: LanguageName(named(strings, each.first)?),
                    path: FilePath(named(strings, each.path)?),
                    reason: ToolMessage(named(strings, each.said)?),
                })
            })
            .collect::<Result<_, postcard::Error>>()?;
        let unknowns = tables
            .unknowns
            .iter()
            .map(|each| unknown_from(strings, each))
            .collect::<Result<_, postcard::Error>>()?;
        let skipped = tables
            .skipped
            .iter()
            .map(|each| {
                let said = named(strings, each.said)?;
                let reason = [SkipReason::SyntaxRejected]
                    .into_iter()
                    .find(|candidate| match candidate {
                        SkipReason::SyntaxRejected => said.as_ref() == SYNTAX_REJECTED,
                    })
                    .ok_or(postcard::Error::DeserializeBadEncoding)?;
                Ok(Skipped {
                    rule: RuleId(named(strings, each.first)?),
                    path: FilePath(named(strings, each.path)?),
                    reason,
                })
            })
            .collect::<Result<_, postcard::Error>>()?;
        let findings = findings_from(
            &tables.strings,
            &tables.findings,
            &tables.finding_keys,
            &tables.finding_values,
            files.len(),
        )?;
        Ok(Self {
            rules,
            files,
            frontiers,
            findings,
            unread,
            refused,
            unknowns,
            skipped,
            consulted,
            consulted_absent,
            project_observations,
            handed: StoredHanded::default(),
        })
    }
}

/// Decode rule identities and crossing policies from the interned snapshot tables.
pub(in crate::outbound::lint_store::adapters::postcard) fn rules_from(
    tables: &TablesSchema<'_>,
) -> Result<Vec<StoredRule>, postcard::Error> {
    let strings = &tables.strings;
    tables
        .rules
        .iter()
        .map(|each| {
            Ok(StoredRule {
                rule: RuleId(named(strings, each.rule)?),
                digest: Digest(each.digest),
                crossing: if each.across {
                    Crossing::Across
                } else {
                    Crossing::Within
                },
            })
        })
        .collect::<Result<_, postcard::Error>>()
}

/// The tables of `stored` but what its files handed, as their section's bytes.
pub(in crate::outbound::lint_store::adapters::postcard) fn tables_bytes(
    stored: &Stored,
) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_stdvec(&TablesSchema::from(stored))
}

/// The tables the section's bytes hold; empty when they do not read back as the tables.
pub(in crate::outbound::lint_store::adapters::postcard) fn tables_of(bytes: &[u8]) -> Stored {
    postcard::from_bytes::<TablesSchema<'_>>(bytes)
        .and_then(Stored::try_from)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
