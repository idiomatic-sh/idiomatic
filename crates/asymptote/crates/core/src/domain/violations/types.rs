//! The types of the `violations` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

/// The id a tool is configured under in `asymptote.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolId(pub Box<str>);

/// The rule a tool reported, as it names it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(pub Box<str>);

/// A path relative to the product's root, `/`-separated, with no `.` segment and every `..` resolved that can be; an
/// absolute path keeps its leading `/`, so it never shares a key with a relative one.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPath(pub Box<str>);

/// A level as its word is written: `note`, `warning` or `error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LevelWord<'a>(pub &'a str);

/// How serious the tool considered a violation: SARIF's levels, `none` read as an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// A note.
    Note,
    /// A warning.
    Warning,
    /// An error.
    Error,
}

/// Whether a violation stands, or an in-source ignore suppresses it: an ignore is a violation of its own standing, so
/// the lock holds each one and a new one is new.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Standing {
    /// Reported.
    Reported,
    /// Suppressed in the source, with a reason.
    Suppressed,
}

/// Why an in-source ignore suppresses a violation, as the ignore says it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Justification(pub Box<str>);

/// Whether an in-source ignore suppresses a violation, and why.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suppression {
    /// None does: it is reported.
    Unsuppressed,
    /// One does, for this reason.
    Suppressed(Justification),
}

/// What identifies a violation within its file: the hash of the text of the line it is reported on, or what its tool
/// put in SARIF's `primaryLocationLineHash`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fingerprint(pub u64);

/// Whether a violation has its fingerprint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fingerprinted {
    /// It has.
    Known(Fingerprint),
    /// Its file or line could not be read: it has no key, and is new by default.
    Unknown,
}

/// A one-based line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineNumber(pub u32);

/// A one-based column.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnNumber(pub u32);

/// Where in its file a violation is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Place {
    /// The whole file: no region.
    File,
    /// A line.
    Line(LineNumber),
    /// A line and a column.
    At(LineNumber, ColumnNumber),
}

/// What the tool said.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageText(pub Box<str>);

/// One violation a tool reported.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Violation {
    /// The file, relative to the product's root.
    pub path: RepoPath,
    /// Where in it.
    pub place: Place,
    /// The tool.
    pub tool: ToolId,
    /// The rule.
    pub rule: RuleId,
    /// How serious.
    pub level: Level,
    /// Whether an in-source ignore suppresses it, and why.
    pub suppression: Suppression,
    /// What the tool said.
    pub message: MessageText,
    /// What identifies it within its file.
    pub fingerprint: Fingerprinted,
}

/// What identifies a violation: two are the same exactly when their keys are equal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    /// The tool.
    pub tool: ToolId,
    /// The rule.
    pub rule: RuleId,
    /// The file.
    pub path: RepoPath,
    /// How serious.
    pub level: Level,
    /// Whether an ignore suppresses it.
    pub standing: Standing,
    /// Its fingerprint.
    pub fingerprint: Fingerprint,
}

/// A key's content address: the lowercase hex of the first sixteen bytes of its hash, as v1's lock writes it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub Box<str>);

/// Whether a violation has a key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Keyed {
    /// This one.
    Key(Key),
    /// None: its fingerprint is unknown.
    Unkeyed,
}

/// A file's text, as a violation's line is read from it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceText(pub Box<str>);

/// Whether a path lies within the product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Within {
    /// It is relative and stays beneath the root.
    Inside,
    /// It is absolute, or climbs out of the root.
    Outside,
}

/// Where a path leads once its links are followed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Located {
    /// Beneath the product's root, or nowhere that exists.
    Inside,
    /// Outside it, to this absolute path.
    Outside(RepoPath),
}
