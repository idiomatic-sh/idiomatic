//! The types of the `storing` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::coverage::types::Unknown;
use crate::domain::workers::types::FileOrder;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::records::types::{FindingRows, PlacedRule, Record};
use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::runs::types::Skipped;
use ryo_values::domain::stores::types::{BucketsWanted, KeptRows, Stored, StoredFile, StoredRule};

/// What a run gives its store: the tables it keeps, and what the store keeps of the words as they were.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToStore {
    /// The tables, with the words of the files the run read.
    pub stored: Stored,
    /// What the store keeps of the words as they were.
    pub kept: KeptRows,
}

/// What a run does with what the last run kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// Every file is read again and every rule run again: nothing was kept, or it was kept for other rules or other files, or a file
    /// that changed changed what the rules that read across files are given.
    Whole,
    /// The files as they were are used again, and so are the findings of the rules that read across files.
    Again,
    /// Some files changed: what each now shows other files decides between the two.
    Check(Vec<FileOrder>),
    /// Files were added or removed while a rule reads across files: the files as they were are used again, and what they handed the
    /// steps that read across files is handed again, so those steps are made again without reading them.
    Cross,
}

/// Which rules that read across files a run makes again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MadeAgain {
    /// None: what they found is as the store kept it.
    Nothing,
    /// Every one: what a file shows the project or every rule changed, or files were added or removed.
    All,
    /// These, by their place, in order: what the changed files hand them changed, and nothing else did.
    These(Vec<PlacedRule>),
}

/// What a run keeps in its store, before it is made into the store's tables.
pub(crate) struct ToKeep<'a> {
    /// The rules, with their digests.
    pub rules: Vec<StoredRule>,
    /// What the run found of each file, in the order of the paths.
    pub records: Vec<Record>,
    /// The findings of the rules that read across files.
    pub later: &'a FindingRows,
    /// The manifests the languages' tools refused.
    pub refused: &'a [ManifestRefused],
    /// What the rules could not decide.
    pub unknowns: &'a [Unknown],
    /// Rules skipped for files whose syntax they require was rejected.
    pub skipped: &'a [Skipped],
    /// The files the project's lookup read beyond the manifests, with their stamps and digests, in the order of their paths.
    pub consulted: &'a [StoredFile],
    /// The files the lookup asked for and found none of, in the order of their paths.
    pub consulted_absent: &'a [FilePath],
}

/// How much of what the last run kept a run uses again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reuse {
    /// Nothing: every file is read and every rule run.
    Whole,
    /// The files as they were, and the findings of the rules that read across files.
    Again,
    /// The files as they were, and what they handed the steps that read across files, which are made again for the rules named;
    /// the others' findings are as the store kept them.
    Cross(MadeAgain),
}

/// What a run uses again of what the last kept.
#[derive(Debug)]
pub(crate) struct Reused {
    /// How much it uses again.
    pub again: Reuse,
    /// The records of the files as the store kept them, with those of the changed files read before the run, in the order of their
    /// paths: none when nothing is used again.
    pub records: Vec<Record>,
    /// What the rules could not decide in the changed files read before the run.
    pub unknowns: Vec<Unknown>,
    /// Rules skipped for files whose syntax they require was rejected.
    pub skipped: Vec<Skipped>,
    /// The files whose stored rows are theirs no more, in the order of their paths.
    pub read_again: Vec<FilePath>,
}

/// The changed files, read on their own before the run to see whether what they show other files changed: what the rules found of
/// each, in the order of their paths, and what the rules could not decide in them.
#[derive(Debug, Default)]
pub(crate) struct ReadAlone {
    /// The record of each file.
    pub records: Vec<Record>,
    /// What the rules could not decide in them.
    pub unknowns: Vec<Unknown>,
    /// Rules skipped for files whose syntax they require was rejected.
    pub skipped: Vec<Skipped>,
}

/// What a run reads again of what the files handed the cross-file steps: the buckets it reads, and what the store kept of them.
#[derive(Debug)]
pub(crate) struct HandedAgain {
    /// The buckets read.
    pub wanted: BucketsWanted,
    /// What the store kept of them.
    pub handed: ryo_values::domain::stores::types::ReadHanded,
}
