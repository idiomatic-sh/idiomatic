// the one schemas.rs the adapter spec places holds the one wire shape of the store: its flat tables, one row per thing, its table of strings, its table of contents, and their conversion from and into the core's tables; a second schemas file is a role no spec places.
//! The shape of the data this adapter does not control: the store as flat tables of numbers, each text once in a table of strings,
//! a row naming it, a file or a finding by its place. The file is sections one after the other: the small tables, what the files
//! handed but their words, then the words of each bucket on its own, and last a table of contents that says where each section is
//! and which build wrote them, so a run reads only the sections it needs.

use super::types::{BucketSpan, FileOffset, SectionLength, SectionSpan, TableOfContents};
use rustc_hash::FxHashMap;
use std::borrow::Cow;

use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::project_context::types::{EntryObservation, ObservedEntryState};
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_relations::domain::relations::types::{Integer, Offset, Span};
use ryo_values::domain::descriptions::types::ColumnName;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::files::types::{FileSize, Nanoseconds};
use ryo_values::domain::records::types::GoverningRules;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::row_texts::types::Cell;
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::row_texts::types::{PlacedColumn, PlacedLine};
use ryo_values::domain::runs::constants::SYNTAX_REJECTED;
use ryo_values::domain::runs::types::{ManifestRefused, RuleId, SkipReason, Skipped};
#[cfg(test)]
use ryo_values::domain::stored_facts::types::PackedWords;
use ryo_values::domain::stored_facts::types::{
    ArenaBytes, Bucket, Digest, FileNotes, LanguageNumber, NoteBytes, PartialAt, PartialCause,
    PartialKind, PartialScope, ReadFailure, StoredPartial,
};
use ryo_values::domain::stored_findings::types::{
    FindingMade, FindingPlace, StoredCell, StoredFileNumber, StoredFinding, StoredPlace,
    StoredUnknown, StoredUnread, StoredValue, UnknownText,
};
use ryo_values::domain::stores::types::{
    Crossing, Stored, StoredCovered, StoredFile, StoredFrontier, StoredHanded, StoredNotes,
    StoredRule, StoredText,
};

#[cfg(test)]
use ryo_values::domain::stores::types::StoredWords;

/// A text, by its place in the table of strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct TextSchema(u32);

/// One exact project-entry observation retained for warm context validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct EntrySchema {
    path: TextSchema,
    state: EntryStateSchema,
}

/// Stable entry-kind tags in the checked store version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum EntryStateSchema {
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
pub(super) struct RuleSchema {
    /// Its id.
    rule: TextSchema,
    /// What its query, parameters and files came to.
    digest: [u8; 16],
    /// Whether it reads across files.
    across: bool,
}

/// A file: one row for each, numbered by its place in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct FileSchema {
    /// Explicit permission observation in this versioned store format.
    executable: ExecutableSchema,
    /// Its path.
    path: TextSchema,
    /// What its bytes came to.
    digest: [u8; 16],
    /// What it shows other files came to.
    interface: [u8; 16],
    /// What its notes and its partly read relations came to.
    noted: [u8; 16],
    /// Its size when it was read.
    size: u64,
    /// When it was last written, when it was read.
    modified: u64,
}

/// What a file hands one rule that reads across files: one row for each file and rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct FrontierSchema {
    /// The file, by its place among the files.
    file: u32,
    /// The rule, by its place among the rules.
    rule: u32,
    /// What the file hands it.
    digest: [u8; 16],
}

/// A value a finding holds, typed: a file or a text by its place in the table of strings, a whole number, or a stretch of a file by
/// its two ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum CellSchema {
    /// A file, by its path.
    File(TextSchema),
    /// A text.
    Text(TextSchema),
    /// A whole number.
    Number(i64),
    /// A stretch of a file: where it starts and where it ends.
    Span(u32, u32),
}

/// A finding: one row for each, numbered by its place in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct FindingSchema {
    /// The rule.
    rule: TextSchema,
    /// The file whose visit made it, by its place among the files; none for a rule that reads across files.
    file: Option<u32>,
    /// Its line and character; none when it has no place.
    place: Option<(u32, u32)>,
    /// What it is about; none when it reads as nothing.
    subject: Option<CellSchema>,
}

/// One value of a finding's key: one row for each, in the order of the findings and, in a finding, of its key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct KeySchema {
    /// The finding, by its place among the findings.
    finding: u32,
    /// The value; none when it reads as nothing.
    value: Option<CellSchema>,
}

/// One column of a finding: one row for each, in the order of the findings and, in a finding, of its columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct ValueSchema {
    /// The finding, by its place among the findings.
    finding: u32,
    /// The column's heading.
    column: TextSchema,
    /// Its value; none when it reads as nothing.
    value: Option<CellSchema>,
}

/// A file no language read: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct UnreadSchema {
    /// The file, by its place among the files.
    file: u32,
    /// What the language said when it failed; none when no language reads such a file.
    failure: Option<TextSchema>,
}

/// A manifest a language's tools refused, or something a rule could not decide: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct TripleSchema {
    /// The language, or the rule.
    first: TextSchema,
    /// The manifest, or the file whose relation is read in part.
    path: TextSchema,
    /// What the tool said, or what is partial there and why.
    said: TextSchema,
}

/// The small tables of the store: what a warm run reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct TablesSchema<'a> {
    /// Every text the rows name, each once.
    #[serde(borrow)]
    strings: Vec<Cow<'a, str>>,
    /// The rules.
    rules: Vec<RuleSchema>,
    /// The files.
    files: Vec<FileSchema>,
    /// What each file hands each rule that reads across files.
    frontiers: Vec<FrontierSchema>,
    /// The findings.
    findings: Vec<FindingSchema>,
    /// The values of the findings' keys.
    finding_keys: Vec<KeySchema>,
    /// The findings' columns.
    finding_values: Vec<ValueSchema>,
    /// The files no language read.
    unread: Vec<UnreadSchema>,
    /// The manifests the tools refused.
    refused: Vec<TripleSchema>,
    /// What the rules could not decide.
    unknowns: Vec<TripleSchema>,
    /// The files the project's lookup read beyond the manifests, as the files are kept.
    consulted: Vec<FileSchema>,
    /// The files the lookup asked for and found none of, as texts.
    consulted_absent: Vec<TextSchema>,
    /// Candidate entries used for project membership, in request order.
    project_observations: Vec<EntrySchema>,
    /// Rules explicitly skipped because their required syntax was rejected.
    #[serde(default)]
    skipped: Vec<TripleSchema>,
}

/// Bytes of a file only the core reads, its names' text: one row for each file that has some.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct BytesSchema<'a> {
    /// The file, by its place among the files.
    file: u32,
    /// The bytes.
    #[serde(borrow, serialize_with = "as_bytes")]
    bytes: Cow<'a, [u8]>,
}

/// What a file's language noted of it, which only that language reads: one row for each file that has some.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct NotesSchema<'a> {
    /// The file, by its place among the files.
    file: u32,
    /// The language that wrote the notes, by its place among the ones linked.
    language: u32,
    /// The notes.
    #[serde(borrow, serialize_with = "as_bytes")]
    bytes: Cow<'a, [u8]>,
}

/// The rules that govern a file read in part: one row for each such file.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct CoveredSchema<'a> {
    /// The file, by its place among the files.
    file: u32,
    /// The rules, by their place.
    governing: Cow<'a, [u32]>,
}

/// A relation of a file read in part: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct PartialSchema {
    /// The file, by its place among the files.
    file: u32,
    /// The relation.
    relation: TextSchema,
    /// Why it is read in part.
    cause: TextSchema,
    /// The kind of the rows that could be missing; empty when any could be.
    lacking: TextSchema,
    /// The file the rows missing are about.
    at: u32,
}

/// Typed named-root readiness beside the existing frontier bucket rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum ObservationReadinessSchema {
    Unknown(TextSchema),
    NoQuery,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct ObservationSchema {
    file: u32,
    version: u32,
    layout: [u8; 16],
    slots: u32,
    complete: Vec<u64>,
    skipped: Vec<u64>,
    refusals: Vec<(u32, ObservationReadinessSchema)>,
}

/// What the files handed the steps that read across files and the project but their words.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct RestSchema<'a> {
    observations: Vec<ObservationSchema>,
    /// Source identity and coverage for each payload row, in the rest section
    /// so existing bucket byte framing and unchanged-bucket copying stay exact.
    proofs: Vec<FrontierProofSchema>,
    /// Every text the rows name, each once.
    #[serde(borrow)]
    strings: Vec<Cow<'a, str>>,
    /// The text each file's names made.
    #[serde(borrow)]
    texts: Vec<BytesSchema<'a>>,
    /// What each file's language noted of it.
    #[serde(borrow)]
    notes: Vec<NotesSchema<'a>>,
    /// The rules that govern each file read in part.
    covered: Vec<CoveredSchema<'a>>,
    /// The relations of each file read in part.
    partials: Vec<PartialSchema>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum FrontierAffectedSchema {
    CurrentFile,
    Repository,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum FrontierCoverageSchema {
    Complete,
    Partial(Vec<FrontierAffectedSchema>),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct FrontierProofSchema {
    file: u32,
    bucket: u32,
    rule: u32,
    input: u32,
    rule_digest: [u8; 16],
    coverage: FrontierCoverageSchema,
}

/// The words one file handed one bucket: one row for each file, in a section of the bucket's own.
#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct WordsSchema<'a> {
    /// The file, by its place among the files.
    file: u32,
    /// The words, packed.
    #[serde(borrow, serialize_with = "as_bytes")]
    words: Cow<'a, [u8]>,
}

/// Where a section is in the file, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct SpanSchema {
    /// Where it starts.
    pub start: u64,
    /// How long it is.
    pub length: u64,
}

/// Where the words of one bucket are in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct BucketSpanSchema {
    /// The bucket.
    pub bucket: u32,
    /// Where its section is.
    pub span: SpanSchema,
}

/// The table of contents, last in the file: which build wrote it and where each section is.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct ContentsSchema<'a> {
    /// The build.
    #[serde(borrow)]
    pub build: Cow<'a, str>,
    /// The small tables.
    pub tables: SpanSchema,
    /// What the files handed but their words.
    pub rest: SpanSchema,
    /// The words of each bucket, in the order of the buckets.
    pub buckets: Vec<BucketSpanSchema>,
}

/// Writes `bytes` as one run of bytes, not a byte at a time: the same bytes in the file, written at once.
fn as_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bytes(bytes)
}

/// The table of strings being written: each text once, by its place.
#[derive(Default)]
struct StringsSchema<'a> {
    /// The texts, in the order they were first named.
    texts: Vec<Cow<'a, str>>,
    /// The place of each text.
    places: FxHashMap<&'a str, u32>,
}

/// The place of `text` in `strings`, which gains it when it is not there.
fn text<'a>(strings: &mut StringsSchema<'a>, text: &'a str) -> TextSchema {
    if let Some(place) = strings.places.get(text) {
        return TextSchema(*place);
    }
    let place = u32::try_from(strings.texts.len()).unwrap_or(u32::MAX);
    strings.texts.push(Cow::Borrowed(text));
    let _: Option<u32> = strings.places.insert(text, place);
    TextSchema(place)
}

/// The text a row names; a refusal when the table of strings has none there.
fn named(strings: &[Cow<'_, str>], at: TextSchema) -> Result<Box<str>, postcard::Error> {
    strings
        .get(usize::try_from(at.0).unwrap_or(usize::MAX))
        .map(|each| Box::from(&**each))
        .ok_or(postcard::Error::DeserializeBadEncoding)
}

/// The row of a typed value, its text gained by `strings` when it has some.
fn cell_schema<'a>(strings: &mut StringsSchema<'a>, cell: &'a Cell) -> CellSchema {
    match cell {
        Cell::File(path) => CellSchema::File(text(strings, path)),
        Cell::Text(each) => CellSchema::Text(text(strings, each)),
        Cell::Number(number) => CellSchema::Number(number.0),
        Cell::Span(span) => CellSchema::Span(span.start.0, span.end.0),
    }
}

/// The typed value a row names; a refusal when the table of strings has none of its text.
fn cell_from(strings: &[Cow<'_, str>], cell: CellSchema) -> Result<Cell, postcard::Error> {
    Ok(match cell {
        CellSchema::File(path) => Cell::File(named(strings, path)?),
        CellSchema::Text(each) => Cell::Text(named(strings, each)?),
        CellSchema::Number(number) => Cell::Number(Integer(number)),
        CellSchema::Span(start, end) => Cell::Span(Span {
            start: Offset(start),
            end: Offset(end),
        }),
    })
}

/// A file a row names, by its place among `files` files; a refusal when there is none there.
fn file_number(files: usize, at: u32) -> Result<StoredFileNumber, postcard::Error> {
    if usize::try_from(at).is_ok_and(|place| place < files) {
        Ok(StoredFileNumber(at))
    } else {
        Err(postcard::Error::DeserializeBadEncoding)
    }
}

/// Wire representation of permission knowledge; no integer sentinel means false.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum ExecutableSchema {
    Unknown,
    Unsupported,
    Posix(bool),
}

/// The row of a file, its path named in the table of strings.
fn file_schema<'a>(strings: &mut StringsSchema<'a>, file: &'a StoredFile) -> FileSchema {
    FileSchema {
        executable: match file.executable {
            ryo_values::domain::files::types::ExecutablePermission::Unknown => {
                ExecutableSchema::Unknown
            }
            ryo_values::domain::files::types::ExecutablePermission::Unsupported => {
                ExecutableSchema::Unsupported
            }
            ryo_values::domain::files::types::ExecutablePermission::PosixExecutable => {
                ExecutableSchema::Posix(true)
            }
            ryo_values::domain::files::types::ExecutablePermission::PosixNotExecutable => {
                ExecutableSchema::Posix(false)
            }
        },
        path: text(strings, &file.path.0),
        digest: file.digest.0,
        interface: file.interface.0,
        noted: file.noted.0,
        size: file.size.0,
        modified: file.modified.0,
    }
}

/// The file a row is of, its path read from the table of strings.
fn file_from(strings: &[Cow<'_, str>], file: &FileSchema) -> Result<StoredFile, postcard::Error> {
    Ok(StoredFile {
        executable: match file.executable {
            ExecutableSchema::Unknown => {
                ryo_values::domain::files::types::ExecutablePermission::Unknown
            }
            ExecutableSchema::Unsupported => {
                ryo_values::domain::files::types::ExecutablePermission::Unsupported
            }
            ExecutableSchema::Posix(true) => {
                ryo_values::domain::files::types::ExecutablePermission::PosixExecutable
            }
            ExecutableSchema::Posix(false) => {
                ryo_values::domain::files::types::ExecutablePermission::PosixNotExecutable
            }
        },
        path: FilePath(named(strings, file.path)?),
        digest: Digest(file.digest),
        interface: Digest(file.interface),
        noted: Digest(file.noted),
        size: FileSize(file.size),
        modified: Nanoseconds(file.modified),
    })
}

/// The rows of the findings, of their key values and of their headed columns, their texts named in `strings`.
fn findings_schema<'a>(
    strings: &mut StringsSchema<'a>,
    stored: &'a [StoredFinding],
) -> (Vec<FindingSchema>, Vec<KeySchema>, Vec<ValueSchema>) {
    let mut findings = Vec::with_capacity(stored.len());
    let mut finding_keys = Vec::new();
    let mut finding_values = Vec::new();
    for (place, each) in stored.iter().enumerate() {
        findings.push(FindingSchema {
            rule: text(strings, &each.rule.0),
            file: match each.made {
                FindingMade::InFile(file) => Some(file.0),
                FindingMade::AcrossFiles => None,
            },
            place: match each.place {
                FindingPlace::At(at) => Some((at.line.0, at.column.0)),
                FindingPlace::Unplaced => None,
            },
            subject: Option::<&Cell>::from(&each.subject).map(|cell| cell_schema(strings, cell)),
        });
        let finding = u32::try_from(place).unwrap_or(u32::MAX);
        for value in &each.key {
            finding_keys.push(KeySchema {
                finding,
                value: Option::<&Cell>::from(value).map(|cell| cell_schema(strings, cell)),
            });
        }
        for value in &each.values {
            finding_values.push(ValueSchema {
                finding,
                column: text(strings, &value.column.0),
                value: Option::<&Cell>::from(&value.value).map(|cell| cell_schema(strings, cell)),
            });
        }
    }
    (findings, finding_keys, finding_values)
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
            .map(|each| TripleSchema {
                first: text(&mut strings, &each.rule.0),
                path: text(&mut strings, &each.path.0),
                said: text(&mut strings, &each.message.0),
            })
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

/// The findings of the tables, with their columns, of a store of `files` files.
fn findings_from(
    tables: &TablesSchema<'_>,
    files: usize,
) -> Result<Vec<StoredFinding>, postcard::Error> {
    let strings = &tables.strings;
    let mut findings = Vec::with_capacity(tables.findings.len());
    for each in &tables.findings {
        findings.push(StoredFinding {
            rule: RuleId(named(strings, each.rule)?),
            made: match each.file {
                Some(file) => FindingMade::InFile(file_number(files, file)?),
                None => FindingMade::AcrossFiles,
            },
            place: each.place.map_or(FindingPlace::Unplaced, |(line, column)| {
                FindingPlace::At(StoredPlace {
                    line: PlacedLine(line),
                    column: PlacedColumn(column),
                })
            }),
            subject: StoredCell::from(
                each.subject
                    .map(|cell| cell_from(strings, cell))
                    .transpose()?,
            ),
            key: Vec::new(),
            values: Vec::new(),
        });
    }
    for each in &tables.finding_keys {
        let finding = findings
            .get_mut(usize::try_from(each.finding).unwrap_or(usize::MAX))
            .ok_or(postcard::Error::DeserializeBadEncoding)?;
        finding.key.push(StoredCell::from(
            each.value
                .map(|cell| cell_from(strings, cell))
                .transpose()?,
        ));
    }
    for each in &tables.finding_values {
        let finding = findings
            .get_mut(usize::try_from(each.finding).unwrap_or(usize::MAX))
            .ok_or(postcard::Error::DeserializeBadEncoding)?;
        finding.values.push(StoredValue {
            column: ColumnName(named(strings, each.column)?),
            value: StoredCell::from(
                each.value
                    .map(|cell| cell_from(strings, cell))
                    .transpose()?,
            ),
        });
    }
    Ok(findings)
}

/// Decode rule identities and crossing policies from the interned snapshot tables.
fn rules_from(tables: &TablesSchema<'_>) -> Result<Vec<StoredRule>, postcard::Error> {
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
            .map(|each| {
                Ok(StoredUnknown {
                    rule: RuleId(named(strings, each.first)?),
                    path: FilePath(named(strings, each.path)?),
                    message: UnknownText(named(strings, each.said)?),
                })
            })
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
        let findings = findings_from(&tables, files.len())?;
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

/// The section of what the files of `stored` handed but their words.
fn proof_schema(
    row: &ryo_values::domain::stores::types::StoredFrontierProof,
) -> FrontierProofSchema {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage};
    FrontierProofSchema {
        file: row.file.0,
        bucket: row.bucket.0,
        rule: row.proof.rule.0,
        input: row.proof.input.0,
        rule_digest: row.proof.rule_digest.0,
        coverage: match &row.proof.coverage {
            FrontierCoverage::Complete => FrontierCoverageSchema::Complete,
            FrontierCoverage::Partial(affected) => FrontierCoverageSchema::Partial(
                affected
                    .iter()
                    .map(|cause| match cause {
                        FrontierAffected::CurrentFile => FrontierAffectedSchema::CurrentFile,
                        FrontierAffected::Repository => FrontierAffectedSchema::Repository,
                    })
                    .collect(),
            ),
        },
    }
}

fn proof_row(
    row: &FrontierProofSchema,
) -> Result<ryo_values::domain::stores::types::StoredFrontierProof, postcard::Error> {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierCoverage, FrontierProof};
    let coverage = match &row.coverage {
        FrontierCoverageSchema::Complete => FrontierCoverage::Complete,
        FrontierCoverageSchema::Partial(affected) => {
            let values = affected
                .iter()
                .map(|cause| match cause {
                    FrontierAffectedSchema::CurrentFile => FrontierAffected::CurrentFile,
                    FrontierAffectedSchema::Repository => FrontierAffected::Repository,
                })
                .collect::<Vec<_>>();
            if values.is_empty() || !values.windows(2).all(|pair| pair[0] < pair[1]) {
                return Err(postcard::Error::DeserializeBadEncoding);
            }
            FrontierCoverage::Partial(values)
        }
    };
    Ok(ryo_values::domain::stores::types::StoredFrontierProof {
        file: StoredFileNumber(row.file),
        bucket: Bucket(row.bucket),
        proof: FrontierProof {
            rule: ryo_values::domain::records::types::ProducerRule(row.rule),
            input: ryo_values::domain::records::types::FrontierOrdinal(row.input),
            rule_digest: Digest(row.rule_digest),
            coverage,
        },
    })
}

fn rest_of<'a>(stored: &'a StoredHanded) -> RestSchema<'a> {
    let mut strings = StringsSchema::default();
    let bytes = |file: StoredFileNumber, bytes: &'a [u8]| BytesSchema {
        file: file.0,
        bytes: Cow::Borrowed(bytes),
    };
    let mut partials = Vec::new();
    for each in &stored.covered {
        for partial in &each.partials {
            partials.push(PartialSchema {
                file: each.file.0,
                relation: text(&mut strings, &partial.relation_name.0),
                cause: text(&mut strings, &partial.cause.0),
                lacking: text(&mut strings, &partial.lacking_kind.0),
                at: match partial.at {
                    PartialScope::OneFile(file) => file.0,
                    PartialScope::Repository => u32::MAX,
                },
            });
        }
    }
    let observations = stored
        .observations
        .iter()
        .map(|each| ObservationSchema {
            file: each.file.0,
            version: each.root.version.0,
            layout: each.root.layout.0.0,
            slots: each.root.slots.0,
            complete: each.root.complete.0.clone(),
            skipped: each.root.skipped.0.clone(),
            refusals: each
                .root
                .refusals
                .iter()
                .map(|refusal| {
                    (
                        refusal.slot.0,
                        match &refusal.reason {
                            ryo_values::domain::records::types::FrontierRefusalReason::Unknown(
                                reason,
                            ) => ObservationReadinessSchema::Unknown(text(&mut strings, &reason.0)),
                            ryo_values::domain::records::types::FrontierRefusalReason::NoQuery => {
                                ObservationReadinessSchema::NoQuery
                            }
                        },
                    )
                })
                .collect(),
        })
        .collect();
    RestSchema {
        observations,
        proofs: stored.proofs.iter().map(proof_schema).collect(),
        texts: stored
            .texts
            .iter()
            .map(|each| bytes(each.file, &each.text.0))
            .collect(),
        notes: stored
            .notes
            .iter()
            .map(|each| NotesSchema {
                file: each.file.0,
                language: u32::try_from(each.notes.language.0).unwrap_or(u32::MAX),
                bytes: Cow::Borrowed(each.notes.notes.0.as_slice()),
            })
            .collect(),
        covered: stored
            .covered
            .iter()
            .map(|each| CoveredSchema {
                file: each.file.0,
                governing: Cow::Borrowed(&each.governing.0),
            })
            .collect(),
        partials,
        strings: strings.texts,
    }
}

/// What the files handed but their words, from its section.
fn handed_of(rest: &RestSchema<'_>) -> Result<StoredHanded, postcard::Error> {
    let strings = &rest.strings;
    let mut proofs = rest
        .proofs
        .iter()
        .map(proof_row)
        .collect::<Result<Vec<_>, _>>()?;
    proofs.sort_by_key(|row| (row.file.0, row.bucket.0));
    if proofs
        .windows(2)
        .any(|pair| pair[0].file == pair[1].file && pair[0].bucket == pair[1].bucket)
    {
        return Err(postcard::Error::DeserializeBadEncoding);
    }
    let mut covered = rest
        .covered
        .iter()
        .map(|each| StoredCovered {
            file: StoredFileNumber(each.file),
            governing: GoverningRules(each.governing.to_vec()),
            partials: Vec::new(),
        })
        .collect::<Vec<_>>();
    for each in &rest.partials {
        let at = covered
            .binary_search_by(|row| row.file.0.cmp(&each.file))
            .map_err(|_not_covered| postcard::Error::DeserializeBadEncoding)?;
        if let Some(row) = covered.get_mut(at) {
            row.partials.push(StoredPartial {
                relation_name: RelationName(named(strings, each.relation)?),
                cause: PartialCause(named(strings, each.cause)?),
                lacking_kind: PartialKind(named(strings, each.lacking)?),
                at: if each.at == u32::MAX {
                    PartialScope::Repository
                } else {
                    PartialScope::OneFile(PartialAt(each.at))
                },
            });
        }
    }
    Ok(StoredHanded(
        ryo_values::domain::stores::types::HandedRows {
            proofs,
            observations: rest
                .observations
                .iter()
                .map(|each| {
                    use ryo_values::domain::records::types::ObservationReadinessCount;
                    use ryo_values::domain::records::types::ObservationReadinessLayout;
                    use ryo_values::domain::records::types::ObservationReadinessSlot;
                    use ryo_values::domain::records::types::ObservationReadinessVersion;
                    use ryo_values::domain::records::types::ObservationReason;
                    use ryo_values::domain::records::types::{
                        FrontierReadiness, FrontierRefusal, FrontierRefusalReason,
                    };
                    Ok(ryo_values::domain::stores::types::StoredFrontierReadiness {
                        file: StoredFileNumber(each.file),
                        root: std::sync::Arc::new(FrontierReadiness {
                            version: ObservationReadinessVersion(each.version),
                            layout: ObservationReadinessLayout(Digest(each.layout)),
                            slots: ObservationReadinessCount(each.slots),
                            complete: ryo_values::domain::records::types::ReadinessBits(
                                each.complete.clone(),
                            ),
                            skipped: ryo_values::domain::records::types::ReadinessBits(
                                each.skipped.clone(),
                            ),
                            refusals: each
                                .refusals
                                .iter()
                                .map(|(slot, reason)| {
                                    Ok(FrontierRefusal {
                                        slot: ObservationReadinessSlot(*slot),
                                        reason: match *reason {
                                            ObservationReadinessSchema::Unknown(reason) => {
                                                FrontierRefusalReason::Unknown(ObservationReason(
                                                    named(strings, reason)?,
                                                ))
                                            }
                                            ObservationReadinessSchema::NoQuery => {
                                                FrontierRefusalReason::NoQuery
                                            }
                                        },
                                    })
                                })
                                .collect::<Result<_, postcard::Error>>()?,
                        }),
                    })
                })
                .collect::<Result<_, postcard::Error>>()?,
            frontier: Vec::new(),
            texts: rest
                .texts
                .iter()
                .map(|each| StoredText {
                    file: StoredFileNumber(each.file),
                    text: ArenaBytes(each.bytes.to_vec()),
                })
                .collect(),
            notes: rest
                .notes
                .iter()
                .map(|each| StoredNotes {
                    file: StoredFileNumber(each.file),
                    notes: FileNotes {
                        language: LanguageNumber(
                            usize::try_from(each.language).unwrap_or(usize::MAX),
                        ),
                        notes: NoteBytes(each.bytes.to_vec()),
                    },
                })
                .collect(),
            covered,
        },
    ))
}

/// The tables of `stored` but what its files handed, as their section's bytes.
pub(super) fn tables_bytes(stored: &Stored) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_stdvec(&TablesSchema::from(stored))
}

/// What the files of `stored` handed but their words, as their section's bytes.
pub(super) fn rest_bytes(stored: &StoredHanded) -> Result<Vec<u8>, postcard::Error> {
    // The room the bytes take, at least, so they are written once and never moved as the section grows.
    let room = stored
        .texts
        .iter()
        .map(|each| each.text.0.len() + 10)
        .sum::<usize>()
        + stored
            .notes
            .iter()
            .map(|each| each.notes.notes.0.len() + 20)
            .sum::<usize>();
    postcard::to_extend(&rest_of(stored), Vec::with_capacity(room))
}

/// The words the files of `stored` handed `bucket`, as its section's bytes; `rows` are the rows of that bucket.
#[cfg(test)]
pub(super) fn bucket_bytes(rows: &[&StoredWords]) -> Result<Vec<u8>, postcard::Error> {
    let words: Vec<WordsSchema<'_>> = rows
        .iter()
        .map(|each| WordsSchema {
            file: each.file.0,
            words: Cow::Borrowed(&each.words.0),
        })
        .collect();
    // The room the bytes take: each row's words and the two numbers before them.
    let room = rows
        .iter()
        .map(|each| each.words.0.len() + 10)
        .sum::<usize>()
        + 10;
    postcard::to_extend(&words, Vec::with_capacity(room))
}

/// The words of a bucket made again, as its section's bytes: those of the files read now, `fresh`, and, of the section `old` the
/// store kept of the bucket, those of each file used again, under its number now (`file_again`, old number to new, in the order of
/// both); in the order of the files. The words of `old` are copied as they are, never decoded. None when no file hands it any.
#[cfg(test)]
pub(super) fn rebuilt_bucket_bytes(
    fresh: &[&StoredWords],
    old: Option<&[u8]>,
    file_again: &[ryo_values::domain::stores::types::FileAgain],
) -> Result<Option<Vec<u8>>, postcard::Error> {
    let mut words: Vec<WordsSchema<'_>> = fresh
        .iter()
        .map(|each| WordsSchema {
            file: each.file.0,
            words: Cow::Borrowed(&each.words.0),
        })
        .collect();
    let kept: Vec<WordsSchema<'_>> = match old {
        Some(bytes) => postcard::from_bytes(bytes)?,
        None => Vec::new(),
    };
    words.extend(kept.into_iter().filter_map(|each| {
        // A file not used again is gone, with its words.
        match file_again.binary_search_by_key(&each.file, |again| again.was.0) {
            Ok(place) => file_again.get(place).map(|again| WordsSchema {
                file: again.now.0,
                words: each.words,
            }),
            Err(_) => None,
        }
    }));
    if words.is_empty() {
        return Ok(None);
    }
    words.sort_by_key(|each| each.file);
    // The room the bytes take: each row's words and the two numbers before them.
    let room = words
        .iter()
        .map(|each| each.words.len() + 10)
        .sum::<usize>()
        + 10;
    postcard::to_extend(&words, Vec::with_capacity(room)).map(Some)
}

impl From<SpanSchema> for SectionSpan {
    fn from(schema: SpanSchema) -> Self {
        Self {
            start: FileOffset(schema.start),
            length: SectionLength(schema.length),
        }
    }
}

impl From<SectionSpan> for SpanSchema {
    fn from(span: SectionSpan) -> Self {
        Self {
            start: span.start.0,
            length: span.length.0,
        }
    }
}

impl From<BucketSpanSchema> for BucketSpan {
    fn from(schema: BucketSpanSchema) -> Self {
        Self {
            bucket: Bucket(schema.bucket),
            span: SectionSpan::from(schema.span),
        }
    }
}

impl From<BucketSpan> for BucketSpanSchema {
    fn from(span: BucketSpan) -> Self {
        Self {
            bucket: span.bucket.0,
            span: SpanSchema::from(span.span),
        }
    }
}

/// The table of contents `build` writes, as its bytes.
pub(super) fn contents_bytes(
    build: &str,
    contents: &TableOfContents,
) -> Result<Vec<u8>, postcard::Error> {
    postcard::to_stdvec(&ContentsSchema {
        build: Cow::Borrowed(build),
        tables: SpanSchema::from(contents.tables),
        rest: SpanSchema::from(contents.rest),
        buckets: contents
            .buckets
            .iter()
            .map(|each| BucketSpanSchema::from(*each))
            .collect(),
    })
}

/// The table of contents `bytes` hold, when `build` wrote them.
pub(super) fn contents(build: &str, bytes: &[u8]) -> Option<TableOfContents> {
    // A table of contents that does not read back is a store kept by another build or cut short: nothing is kept.
    let Ok(contents) = postcard::from_bytes::<ContentsSchema<'_>>(bytes) else {
        return None;
    };
    (*contents.build == *build).then(|| TableOfContents {
        tables: SectionSpan::from(contents.tables),
        rest: SectionSpan::from(contents.rest),
        buckets: contents.buckets.into_iter().map(BucketSpan::from).collect(),
    })
}

/// The tables the section's bytes hold; empty when they do not read back as the tables.
pub(super) fn tables_of(bytes: &[u8]) -> Stored {
    postcard::from_bytes::<TablesSchema<'_>>(bytes)
        .and_then(Stored::try_from)
        .unwrap_or_default()
}

/// What the files handed but their words, from the section's bytes.
pub(super) fn rest_from(bytes: &[u8]) -> Option<StoredHanded> {
    // A section that does not read back holds nothing the run can use again.
    let Ok(rest) = postcard::from_bytes::<RestSchema<'_>>(bytes) else {
        return None;
    };
    match handed_of(&rest) {
        Ok(handed) => Some(handed),
        Err(_) => None,
    }
}

/// The words the files handed `bucket`, from its section's bytes.
#[cfg(test)]
pub(super) fn bucket_from(
    bucket: Bucket,
    bytes: &[u8],
    proofs: &[ryo_values::domain::stores::types::StoredFrontierProof],
) -> Option<Vec<StoredWords>> {
    let Ok(rows) = postcard::from_bytes::<Vec<WordsSchema<'_>>>(bytes) else {
        return None;
    };
    rows.into_iter()
        .map(|each| {
            Some(StoredWords {
                file: StoredFileNumber(each.file),
                bucket,
                proof: proofs
                    .iter()
                    .find(|row| row.file.0 == each.file && row.bucket == bucket)?
                    .proof
                    .clone(),
                words: PackedWords(each.words.into_owned()),
            })
        })
        .collect::<Option<Vec<_>>>()
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
