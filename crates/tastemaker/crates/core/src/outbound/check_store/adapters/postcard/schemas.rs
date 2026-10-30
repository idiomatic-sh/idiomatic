//! The shape of `check`'s store on disk: postcard's bytes of the build that wrote it and of flat, compact tables, a row
//! a thing: `files`, a file the answer rests on each; `findings`, a finding each; and `rules`, a rule that ran each.
//! Every text is kept once, in `strings`, and a row names it by number.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::digests::types::Digest;
use crate::domain::files::types::Stamp;
use crate::domain::findings::types::{
    CheckChoice, CheckName, Column, Finding, FindingRule, Fingerprint, Help, Line, MessageText,
    Place, Reason, Region, RuleId, Suppression,
};
use crate::domain::paths::types::RepoPath;
use crate::outbound::check_store::types::{Checked, CheckedFile, CheckedPath, SourcedFile};

/// The number a text is kept under in `strings`.
type Text = u32;

/// A row of `files`: a file a check's answer rests on, its digest, size and modification time.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct FileSchema {
    /// Its path.
    path: Text,
    /// Its digest.
    digest: Text,
    /// Its size.
    size: u64,
    /// When it was modified, in nanoseconds since the Unix epoch.
    modified: u64,
}

/// A row of `sourced`: a file settled against its text, and what it had before its ignores were applied.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct SourcedSchema {
    /// Its path.
    path: Text,
    /// What it had, as one fingerprint.
    incoming: u64,
}

/// Where a finding is: a whole file, a line, or a region.
#[derive(Clone, Copy, Serialize, Deserialize)]
enum PlaceSchema {
    /// The whole file or directory.
    Whole,
    /// A line.
    Line(u32),
    /// A region: its first line and column, its last line and the column after it.
    At(u32, u32, u32, u32),
}

/// A rule or a check: the rule, or the entry the check is of, and the check's name for a check.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct RuleSchema {
    /// The rule, or the entry.
    rule: Text,
    /// The check's name, for a check.
    check: Option<Text>,
}

/// A row of `findings`: rule, path, key, place, message, help and, for a finding an in-source ignore accepted,
/// its reason.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct FindingSchema {
    /// The rule or check.
    rule: RuleSchema,
    /// The file or directory it is about.
    path: Text,
    /// Its fingerprint.
    key: u64,
    /// Where it is.
    place: PlaceSchema,
    /// What is wrong.
    message: Text,
    /// What to do, when it says.
    help: Option<Text>,
    /// Why an in-source ignore accepts it, for one accepted.
    reason: Option<Text>,
}

/// The store: the build that wrote it, every text, and the tables.
#[derive(Default, Serialize, Deserialize)]
pub(super) struct StoreSchema {
    /// The build that wrote it.
    build: Box<str>,
    /// Every text, each once.
    strings: Vec<Box<str>>,
    /// The files the answer rests on.
    files: Vec<FileSchema>,
    /// The findings.
    findings: Vec<FindingSchema>,
    /// The rules and checks that ran.
    rules: Vec<RuleSchema>,
    /// The rules the ignores were read against, as one digest.
    known: Text,
    /// The files settled against their text, and what each had before its ignores were applied.
    sourced: Vec<SourcedSchema>,
}

/// The `strings` table of a store being written: each text given its number once.
struct StringsSchema {
    /// The texts, in number order.
    strings: Vec<Box<str>>,
    /// Each text's number.
    numbers: HashMap<Box<str>, Text>,
}

impl StringsSchema {
    /// The number of `text`, given it when it has none.
    fn number(&mut self, text: &str) -> Text {
        if let Some(number) = self.numbers.get(text) {
            return *number;
        }
        let number = Text::try_from(self.strings.len()).unwrap_or(Text::MAX);
        self.strings.push(text.into());
        let _: Option<Text> = self.numbers.insert(text.into(), number);
        number
    }
}

/// What a place is kept as.
const fn place_schema(place: Place) -> PlaceSchema {
    match place {
        Place::Whole => PlaceSchema::Whole,
        Place::Line(line) => PlaceSchema::Line(line.0),
        Place::At(at) => PlaceSchema::At(
            at.start_line.0,
            at.start_column.0,
            at.end_line.0,
            at.end_column.0,
        ),
    }
}

/// The place a kept place names.
const fn place_of(place: PlaceSchema) -> Place {
    match place {
        PlaceSchema::Whole => Place::Whole,
        PlaceSchema::Line(line) => Place::Line(Line(line)),
        PlaceSchema::At(start_line, start_column, end_line, end_column) => Place::At(Region {
            start_line: Line(start_line),
            start_column: Column(start_column),
            end_line: Line(end_line),
            end_column: Column(end_column),
        }),
    }
}

impl StoreSchema {
    /// The text numbered `number`.
    fn text(&self, number: Text) -> Option<&str> {
        self.strings
            .get(usize::try_from(number).ok()?) // ignore: idiomatic.arch/no-ok-on-result - a number past what usize holds names no text, and a row naming none reads the whole store as none
            .map(|text| &**text)
    }

    /// The rule or check a row names; none when a text does not read.
    fn rule(&self, row: RuleSchema) -> Option<FindingRule> {
        Some(FindingRule {
            rule: RuleId(self.text(row.rule)?.into()),
            check: match row.check {
                Some(check) => CheckChoice::Named(CheckName(self.text(check)?.into())),
                None => CheckChoice::Rule,
            },
        })
    }

    /// The finding a row holds; none when a text does not read.
    fn finding(&self, row: &FindingSchema) -> Option<Finding> {
        Some(Finding {
            path: RepoPath(self.text(row.path)?.into()),
            place: place_of(row.place),
            rule: self.rule(row.rule)?,
            message: MessageText(self.text(row.message)?.into()),
            help: match row.help {
                Some(said) => Help::Said(MessageText(self.text(said)?.into())),
                None => Help::Silent,
            },
            fingerprint: Fingerprint(row.key),
        })
    }

    /// The check's answer; none when a row does not read.
    fn checked(&self) -> Option<Checked> {
        let mut findings = Vec::new();
        let mut suppressed = Vec::new();
        for row in &self.findings {
            let finding = self.finding(row)?;
            match row.reason {
                Some(reason) => suppressed.push(Suppression {
                    finding,
                    reason: Reason(self.text(reason)?.into()),
                }),
                None => findings.push(finding),
            }
        }
        let file = |row: &FileSchema| {
            Some(CheckedFile {
                path: CheckedPath(self.text(row.path)?.into()),
                digest: Digest(self.text(row.digest)?.into()),
                stamp: Stamp {
                    size: crate::domain::files::types::FileSize(row.size),
                    modified: crate::domain::files::types::ModifiedTime(row.modified),
                },
            })
        };
        Some(Checked {
            files: self.files.iter().map(file).collect::<Option<_>>()?,
            known: Digest(self.text(self.known)?.into()),
            sourced: self
                .sourced
                .iter()
                .map(|row| {
                    Some(SourcedFile {
                        path: CheckedPath(self.text(row.path)?.into()),
                        incoming: Fingerprint(row.incoming),
                    })
                })
                .collect::<Option<_>>()?,
            findings,
            suppressed,
            evaluated: self
                .rules
                .iter()
                .map(|rule| self.rule(*rule))
                .collect::<Option<_>>()?,
        })
    }

    /// The store of `build` holding `checked`, each text kept once.
    fn with(build: &crate::domain::settings::types::Build, checked: &Checked) -> Self {
        let mut numbers = StringsSchema {
            strings: Vec::new(),
            numbers: HashMap::new(),
        };
        let mut file = |file: &CheckedFile| FileSchema {
            path: numbers.number(&file.path.0),
            digest: numbers.number(&file.digest.0),
            size: (file.stamp.size).0,
            modified: (file.stamp.modified).0,
        };
        let files = checked.files.iter().map(&mut file).collect();
        let sourced = checked
            .sourced
            .iter()
            .map(|each| SourcedSchema {
                path: numbers.number(&each.path.0),
                incoming: each.incoming.0,
            })
            .collect();
        let known = numbers.number(&checked.known.0);
        let rule = |rule: &FindingRule, strings: &mut StringsSchema| RuleSchema {
            rule: strings.number(&rule.rule.0),
            check: match &rule.check {
                CheckChoice::Rule => None,
                CheckChoice::Named(check) => Some(strings.number(&check.0)),
            },
        };
        let mut row = |finding: &Finding, reason: Option<&Reason>| FindingSchema {
            rule: rule(&finding.rule, &mut numbers),
            path: numbers.number(&finding.path.0),
            key: finding.fingerprint.0,
            place: place_schema(finding.place),
            message: numbers.number(&finding.message.0),
            help: match &finding.help {
                Help::Silent => None,
                Help::Said(said) => Some(numbers.number(&said.0)),
            },
            reason: reason.map(|accepted| numbers.number(&accepted.0)),
        };
        let mut findings = Vec::with_capacity(checked.findings.len() + checked.suppressed.len());
        for finding in &checked.findings {
            findings.push(row(finding, None));
        }
        for kept in &checked.suppressed {
            findings.push(row(&kept.finding, Some(&kept.reason)));
        }
        let rules = checked
            .evaluated
            .iter()
            .map(|each| RuleSchema {
                rule: numbers.number(&each.rule.0),
                check: match &each.check {
                    CheckChoice::Rule => None,
                    CheckChoice::Named(check) => Some(numbers.number(&check.0)),
                },
            })
            .collect();
        Self {
            build: build.0.clone(),
            strings: numbers.strings,
            files,
            findings,
            rules,
            known,
            sourced,
        }
    }
}

/// A kept store as a run reads it: its bytes, and the build that runs.
pub(super) struct ReadSchema<'a> {
    /// The bytes kept.
    pub bytes: &'a [u8],
    /// The build that runs.
    pub build: &'a crate::domain::settings::types::Build,
}

/// Why a kept store holds nothing for the run that reads it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ReadSchemaError {
    /// Another build wrote it.
    OtherBuild,
    /// Its bytes are no store, or a row names a text or a row it does not hold.
    Unread,
}

impl TryFrom<ReadSchema<'_>> for Checked {
    type Error = ReadSchemaError;

    fn try_from(read: ReadSchema<'_>) -> Result<Self, Self::Error> {
        let kept: StoreSchema =
            postcard::from_bytes(read.bytes).map_err(|_postcard_error| ReadSchemaError::Unread)?; // bytes that are no store are `Unread`: the store is rebuilt, never migrated
        if *kept.build != *read.build.0 {
            return Err(ReadSchemaError::OtherBuild);
        }
        kept.checked().ok_or(ReadSchemaError::Unread)
    }
}

/// The answer the kept store `bytes` holds when `build` wrote it; none when another build wrote it or it does not read.
pub(super) fn checked(
    bytes: &[u8],
    build: &crate::domain::settings::types::Build,
) -> Option<Checked> {
    Checked::try_from(ReadSchema { bytes, build }).ok() // ignore: idiomatic.arch/no-ok-on-result - a store another build wrote, or that does not read, holds nothing for this run (the doc above)
}

/// The bytes of the store of `build` holding `checked`, each text kept once.
///
/// # Errors
///
/// When postcard cannot write them.
pub(super) fn bytes(
    build: &crate::domain::settings::types::Build,
    checked: &Checked,
) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_allocvec(&StoreSchema::with(build, checked))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
