//! The types of the `manifest` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use crate::domain::violations::types::ToolId;

/// The product's root: the directory holding its `asymptote.toml`, absolute. Every path asymptote keys is relative
/// to it, and every tool runs in it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductRoot(pub Box<str>);

/// One argument of a command line.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Argument(pub Box<str>);

/// A command line: the program, then its arguments.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Argv(pub Vec<Argument>);

/// The command that fixes what a tool reports, when it has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fix {
    /// This command.
    Command(Argv),
    /// None.
    Absent,
}

/// What a tool's run is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Output {
    /// A SARIF 2.1.0 log on stdout: the run completed when it parses, and an exit of 2 or more is a failure.
    Sarif,
    /// Its exit status alone: 0 passes, anything else fails.
    ExitStatus,
    /// Bencher Metric Format on stdout: the run completed when it exits 0 and its stdout parses.
    Bmf,
}

/// A label a tool carries, which `check --groups` selects by: asymptote gives no group a meaning.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Group(pub Box<str>);

/// Whether a check runs a tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Running {
    /// Its group is selected.
    Runs,
    /// Its group is not.
    Skips,
}

/// Which groups a check runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Groups {
    /// Every tool.
    All,
    /// Only the tools of these groups.
    Only(Vec<Group>),
}

/// A number of seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Seconds(pub u64);

/// A git pathspec, relative to the product's root, naming files a tool reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pathspec(pub Box<str>);

/// One tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tool {
    /// Its id, which keys its violations.
    pub id: ToolId,
    /// What runs it.
    pub run: Argv,
    /// What fixes what it reports.
    pub fix: Fix,
    /// What its run is read as.
    pub output: Output,
    /// What it is for.
    pub group: Group,
    /// How long it may run.
    pub timeout: Seconds,
    /// The files it reads, which say whether its last run still holds: the product's own when the file names none.
    pub inputs: Vec<Pathspec>,
}

/// The name of a git hook as the command line gives it, before it is known to be one.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookWord(pub Box<str>);

/// The name of a git hook: one a git client runs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookName(pub Box<str>);

/// The hooks that are mapped, listed for a refusal to name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HookList(pub Box<str>);

/// A git hook and the groups the check it runs selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hook {
    /// The hook.
    pub name: HookName,
    /// The groups its check runs.
    pub groups: Vec<Group>,
}

/// A glob of directories, relative to the workspace root, that each hold a member product's `asymptote.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Member(pub Box<str>);

/// A release of asymptote, as `asymptote --version` names it: `0.1.0`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(pub Box<str>);

/// Whether a manifest pins the asymptote a repository needs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Pin {
    /// It names none.
    #[default]
    Unpinned,
    /// It names this one.
    Pinned(Version),
}

/// One numeric part of a version's `MAJOR.MINOR.PATCH`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VersionPart(pub u64);

/// The text of a pre-release: what follows a version's first `-`, up to any `+`, never empty.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreReleaseTag(pub Box<str>);

/// Whether a version names a pre-release.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PreRelease {
    /// It names this one.
    Named(PreReleaseTag),
    /// It is a release.
    Absent,
}

/// A version read into its parts, build metadata dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    /// Its numeric parts, as many as it writes.
    pub numbers: Vec<VersionPart>,
    /// Its pre-release, or that it is a release.
    pub pre: PreRelease,
}

/// Whether an installed version meets a pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Satisfaction {
    /// It does.
    Met,
    /// It does not.
    Unmet,
}

/// A product's `asymptote.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    /// The asymptote version the repository needs, which the workspace root declares.
    pub asymptote: Pin,
    /// Its tools, in the order they run.
    pub tools: Vec<Tool>,
    /// The members of the workspace it is the root of; none for a product that is not.
    pub members: Vec<Member>,
    /// The hooks it maps to groups, when it is the workspace root; none when it has no `[hooks]` table.
    pub hooks: Vec<Hook>,
}

/// Whether the nearest `asymptote.toml` is one product or a workspace root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// It is one product.
    Product,
    /// It names members: its own tools, then each member's.
    Workspace,
}

/// A product's path from the workspace root: `.` for the root itself.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProductPath(pub Box<str>);

/// One product a ratchet command reaches, and the service that answers for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reached<S> {
    /// Its path from the workspace root: `.` for the root itself.
    pub path: ProductPath,
    /// Its root.
    pub root: ProductRoot,
    /// The service over its own ports.
    pub service: S,
}

/// The products a ratchet command reaches from where it runs: `check`, `up` and `down` reach the same ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reach<S> {
    /// Whether it is one product or a workspace.
    pub layout: Layout,
    /// The products, the root's own tools first, then each member by path.
    pub products: Vec<Reached<S>>,
}
