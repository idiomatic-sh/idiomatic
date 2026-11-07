//! The types of the `findings` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeSet;

use crate::domain::paths::types::RepoPath;

/// A rule's `message` or `help`, naming the query's columns in braces.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageTemplate(pub Box<str>);

/// A rule (`package/rule`), or the entry or file whose built-in checks a finding is of (`package/name`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(pub Box<str>);

/// One of an entry's built-in checks, which Tastemaker makes beside the rules it runs: `count`, `closed`, `coverage`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckName(pub Box<str>);

/// What a finding is of: a rule, or one check of an entry, the two kept apart and joined (`package/name#count`) only when
/// printed.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FindingRule {
    /// The rule, or the entry the check is of.
    pub rule: RuleId,
    /// Whether it is the rule itself or one check of the entry.
    pub check: CheckChoice,
}

/// Whether a finding is of a rule itself, or of one of an entry's built-in checks.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CheckChoice {
    /// The rule itself.
    Rule,
    /// This check of the entry.
    Named(CheckName),
}

/// What a finding is of, as it is printed and as an ignore or a scenario writes it: `package/rule`, or `package/name#count`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleText(pub Box<str>);

/// A line of a file, from one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Line(pub u32);

/// A column of a line, from one, in characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Column(pub u32);

/// A range of lines and columns in a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Region {
    /// The first line.
    pub start_line: Line,
    /// The first column.
    pub start_column: Column,
    /// The last line.
    pub end_line: Line,
    /// The column after the last.
    pub end_column: Column,
}

/// Where in its file a finding is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    /// About the whole file or directory.
    Whole,
    /// At a region of it.
    At(Region),
    /// At a line the engine names, its columns unknown.
    Line(Line),
}

/// What a finding says is wrong.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageText(pub Box<str>);

/// What to do about a finding, when it says.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Help {
    /// It says nothing more.
    Silent,
    /// This instruction.
    Said(MessageText),
}

/// What identifies a finding across edits that do not touch it: its rule, file, message and the text of its first line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(pub u64);

/// One thing a check found: an error, as every finding is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Finding {
    /// The file or directory it is about.
    pub path: RepoPath,
    /// Where in it.
    pub place: Place,
    /// The rule or check.
    pub rule: FindingRule,
    /// What is wrong.
    pub message: MessageText,
    /// What to do.
    pub help: Help,
    /// What identifies it.
    pub fingerprint: Fingerprint,
}

/// Why an in-source ignore accepts a finding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reason(pub Box<str>);

/// A finding an in-source ignore accepted, and the ignore's reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Suppression {
    /// The finding.
    pub finding: Finding,
    /// Why it is accepted.
    pub reason: Reason,
}

/// What one check found, what its ignores suppressed, and every rule and check it ran.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The workspace's name.
    pub workspace: WorkspaceName,
    /// The findings, sorted.
    pub findings: Vec<Finding>,
    /// The suppressed findings, sorted.
    pub suppressed: Vec<Suppression>,
    /// Every rule and check that ran.
    pub evaluated: BTreeSet<FindingRule>,
    /// What each file that was settled against its text had before its ignores were applied, as one fingerprint: while the file and
    /// that are as they were, the next check does not read it.
    pub sourced: std::collections::BTreeMap<RepoPath, Fingerprint>,
}

/// Whether a report fails the check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No finding at `error`.
    Passed,
    /// A finding at `error`.
    Failed,
}

/// The three parts a fingerprint hashes, in order, and how many findings with the same parts came before it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifying {
    /// The rule, file, and the trimmed text of the first line (empty without a region); never the message.
    pub parts: IdentityParts,
    /// How many findings with the same parts came before.
    pub ordinal: Ordinal,
}

/// A count of findings sharing an identity before this one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ordinal(pub u64);

/// A workspace's name, shown in output.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkspaceName(pub Box<str>);

/// The three texts a fingerprint hashes, in order: the rule, the file and the trimmed first line of code.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdentityParts(pub [Box<str>; 3]);

/// The trimmed text of the first line of code a finding covers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FirstLine(pub String);

/// The text a finding's place is read in: the file's contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceText<'a>(pub &'a str);

/// A place in a file's text, in bytes from its start.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteOffset(pub usize);

/// Whether two findings report the same thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sameness {
    /// The same file, place, rule and message.
    Same,
    /// Something differs.
    Different,
}

/// The name of a column a template's placeholder names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Placeholder<'a>(pub &'a str);

/// The value a placeholder is written as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceholderValue(pub String);

/// A message written out from its template.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedText(pub String);

impl From<String> for MessageText {
    fn from(text: String) -> Self {
        Self(text.into())
    }
}

impl From<&str> for MessageText {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::fmt::Display for WorkspaceName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A finding and its independently extracted first line, ready for ordered ordinal assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedIdentity<'a> {
    /// The finding in final report order.
    pub finding: &'a Finding,
    /// Its first code line, or empty for a whole-file finding.
    pub line: FirstLine,
}
