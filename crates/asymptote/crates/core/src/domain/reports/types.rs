//! The types of the `reports` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use std::collections::BTreeMap;

use crate::domain::manifest::types::Pathspec;

use crate::domain::judgement::types::Run;
use crate::domain::perf::types::Measurement;
use crate::domain::renames::types::Head;
use crate::domain::violations::types::{RepoPath, Violation};

/// A file's size, in bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Size(pub u64);

/// A moment, in nanoseconds since the Unix epoch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Moment(pub u128);

/// What a file was when its stamp was taken, as git's index keeps it: its size and modification time, and the stamp.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileRecord {
    /// Its size then.
    pub size: Size,
    /// Its modification time then.
    pub modified: Moment,
    /// The digest of its bytes then.
    pub digest: Digest,
}

/// The record of every file a check stamped, so a file whose size and modification time did not move keeps its
/// stamp without being read again, and the moment the records were taken: a record taken so soon after its file was
/// modified that a change in the same timestamp tick could hide is racy, and the file is read again.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recorded {
    /// Each file's record.
    pub files: BTreeMap<RepoPath, FileRecord>,
    /// When the records were taken: the earliest moment any of them was.
    pub taken: Moment,
}

/// Every input of a check's tools: the files each pathspec names, none when it could not be listed, and each file's
/// stamp.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listed {
    /// The files each pathspec names.
    pub files: BTreeMap<Pathspec, Option<Vec<RepoPath>>>,
    /// Each file's stamp.
    pub stamps: BTreeMap<RepoPath, Digest>,
}

/// What one pathspec of a tool's inputs came to when it ran: the digest of the node it names in the input tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedInput {
    /// The pathspec, as `inputs` writes it.
    pub spec: Pathspec,
    /// The digest of what it names.
    pub digest: Digest,
}

/// What a file is on disk now, without reading it: its size and modification time, or that it is absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileState {
    /// It is there, this size, modified then.
    Present {
        /// Its size.
        size: Size,
        /// Its modification time.
        modified: Moment,
    },
    /// It is not.
    Absent,
}

/// One file's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStat {
    /// The file.
    pub path: RepoPath,
    /// What it is now.
    pub state: FileState,
}

/// The state of files, in the order asked, and the moment they were looked at, before any was.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Each file's state.
    pub files: Vec<FileStat>,
    /// When they were looked at.
    pub taken: Moment,
}

/// The byte strings a digest is taken over, in order: a tool's stamp, or what a pathspec names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HashParts<'a>(pub Vec<&'a [u8]>);

/// Whether a run's inputs could be stamped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stamped {
    /// They were: the digest of the tool's command line and of what each of its pathspecs names.
    Digested(Digest),
    /// They could not be listed: the run is never reused.
    Unstamped,
}

/// A BLAKE3 digest, in lowercase hex: of a file's bytes (or that it is absent), of what a pathspec names, or of what a
/// tool's run rests on.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Digest(pub Box<str>);

/// A file a check's judgement rests on, with its stamp then.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileDigest {
    /// The file.
    pub path: RepoPath,
    /// Its stamp: the digest of its bytes, or that it is absent.
    pub digest: Digest,
}

/// The files of one reading, each with its stamp, in the order they were asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDigests(pub Vec<FileDigest>);

/// Whether a kept run belongs to the check that kept it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keeping {
    /// The check ran this tool, or reused its run: `up` and `down` read it.
    Current,
    /// An earlier check ran it, and this one did not select its tool: kept only for a later check to reuse.
    Retained,
}

/// Full process text retained for diagnosis, never interpreted as findings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticText(pub Box<str>);

/// Whether a failed completion's complete output was retained.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FailureOutput {
    /// No capture exists, including records written before output retention.
    #[default]
    Unavailable,
    /// Both streams were observed; an empty stream is an observed empty value.
    Captured {
        /// Complete standard output, including output preceding a parser failure.
        stdout: DiagnosticText,
        /// Complete standard error, without the display tail's truncation.
        stderr: DiagnosticText,
        /// The execution command and input identity, or explicitly unstamped.
        inputs: Stamped,
    },
}

/// One tool's run, with what it reported and the stamp it ran under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scanned {
    /// Retained failure diagnostics; successful runs do not retain process streams.
    pub failure_output: FailureOutput,
    /// The run.
    pub run: Run,
    /// What it reported.
    pub violations: Vec<Violation>,
    /// The stamp of its inputs.
    pub stamp: Stamped,
    /// What each pathspec of its inputs came to, when they were stamped.
    pub inputs: Vec<NamedInput>,
    /// What it measured: only a Bencher Metric Format tool does.
    pub measurements: Vec<Measurement>,
    /// Whether the check that kept it ran it.
    pub keeping: Keeping,
}

/// Every violation a report's runs reported, and every run.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Observed {
    /// The violations.
    pub violations: Vec<Violation>,
    /// The runs.
    pub runs: Vec<Run>,
}

/// What a check leaves: the commit it ran on and each tool's run. The files its judgement rests on are the records
/// beside it, so `up` and `down` refuse when any of them moved since.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// The commit it ran on.
    pub head: Head,
    /// Each tool's run, in the order they ran.
    pub scanned: Vec<Scanned>,
}
