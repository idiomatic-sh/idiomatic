//! The types of the `module_tree` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeMap;

use ryo::languages::Offset;

use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;

/// One crate's declared module tree, discovered from its root file. A module path is relative to the crate root; the root module is the empty path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleTree {
    /// The directory of the package the crate is of.
    pub package: RepoPath,
    /// The files declaring each module: several when `#[cfg]`-alternate `mod sys;` declare one, and every alternative is a dependency.
    pub modules: BTreeMap<ModulePath, Vec<RepoPath>>,
    /// Every module path declared for each file; `#[path]` can declare one file twice.
    pub files: BTreeMap<RepoPath, Vec<ModulePath>>,
    /// Each module that could not be followed, and why: no file for it, two, or a file already being loaded.
    pub gaps: Vec<Unfollowed>,
    /// The modules declared under `#[cfg(test)]`: they and every module inside them are built for the unit tests alone.
    pub tests: std::collections::BTreeSet<ModulePath>,
}

/// Where a `mod` declaration puts its module: the module's path, and the directories its children and its `#[path]` resolve under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Placing {
    /// The module the declaration makes.
    pub module: ModulePath,
    /// The directory the module's children resolve under.
    pub base: RepoPath,
    /// The directory a `#[path]` inside it resolves under.
    pub path_base: RepoPath,
}

/// How many source files a module has, when it has not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, strum::Display)]
pub(crate) enum SourceCount {
    /// None.
    #[strum(serialize = "no")]
    None,
    /// Two.
    #[strum(serialize = "two")]
    Two,
}

/// Why a module could not be followed: no file for it, two, or a file already being loaded.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Unfollowed {
    /// The module has no source file, or two.
    Files {
        /// The file that declares it.
        file: RepoPath,
        /// The module.
        module: ModuleText,
        /// How many files it has.
        count: SourceCount,
    },
    /// The module's file is already being loaded.
    Loading {
        /// The file that declares it.
        file: RepoPath,
        /// The module.
        module: ModuleText,
        /// The file already being loaded.
        target: RepoPath,
    },
}

impl std::fmt::Display for Unfollowed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Files {
                file,
                module,
                count,
            } => write!(
                f,
                "{file} declares module `{module}` with {count} source files"
            ),
            Self::Loading {
                file,
                module,
                target,
            } => write!(
                f,
                "{file} declares module `{module}` as {target}, which is already being loaded"
            ),
        }
    }
}

/// One segment of a path as the code writes it, or the name of one module: a name, or a keyword (`crate`, `self`, `super`), or a glob.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Segment(pub Box<str>);

impl std::ops::Deref for Segment {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Segment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The path of a module relative to its crate's root; the root module is the empty path.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ModulePath(pub Vec<Segment>);

impl std::ops::Deref for ModulePath {
    type Target = [Segment];

    fn deref(&self) -> &[Segment] {
        &self.0
    }
}

/// A module path as a finding shows it: its segments joined with `::`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ModuleText(pub Box<str>);

impl std::fmt::Display for ModuleText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An inline module block `mod name { … }` that a declaration is written inside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EnclosingBlock {
    /// The module's name.
    pub name: Segment,
    /// The value of its `#[path = "…"]`, when it has one.
    pub path: Section<WrittenPath>,
}

/// One `mod` declaration a file makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModDeclaration {
    /// The inline blocks it is written inside, outermost first.
    pub around: Vec<EnclosingBlock>,
    /// The module's name.
    pub name: Segment,
    /// Whether it is `mod name { … }`, which lives in the file, and not `mod name;`, which lives in another.
    pub body: Body,
    /// The value of its `#[path = "…"]`, when it has one.
    pub path: Section<WrittenPath>,
    /// Whether the module is built always, or only for the package's unit tests.
    pub gate: Gate,
}

/// When a module is built: always, or only under `#[cfg(test)]`, for the package's unit tests, with its dev-dependencies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Gate {
    /// In every build of its crate.
    #[default]
    Always,
    /// Only under `#[cfg(test)]`.
    Test,
}

/// Whether a module is written inline in its file or in a file of its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Body {
    /// Between braces, in the file that declares it.
    Inline,
    /// In a file of its own.
    #[default]
    File,
}

/// A path a file writes: every `use` target, and every outermost path of two segments or more.
#[derive(Clone, Debug, PartialEq, Eq)]
// a written path as the module tree owns it; its look-alike is the notes' borrowed `WrittenNote`, which is converted to this once
pub(crate) struct Written {
    /// The segments, keywords as written and generic arguments left out; a glob ends in `*`.
    pub segments: Vec<Segment>,
    /// The names of the inline modules around it, outermost first.
    pub enclosing: Vec<Segment>,
    /// Where it starts, in bytes.
    pub start: Offset,
    /// Where it ends, in bytes.
    pub end: Offset,
    /// The path as the file writes it, its whitespace left out.
    pub text: Spelling,
    /// The name a `use` leaf binds in its module: its `as` name, or else its last segment; none for a glob, an `as _` or a path
    /// that is not a `use` leaf.
    pub bound: Option<Segment>,
    /// Where a `use` leaf binds its name: the innermost block of statements or inline module around it, by its span. None for a path
    /// that is not a `use` leaf and for a `use` written at the top of its file, which binds in the whole file.
    pub scope: Option<ryo::languages::Span>,
}

/// A path as the file writes it from where it starts to where it ends, its whitespace left out: `crate::a::<T>::b`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Spelling(pub Box<str>);

/// What one source file says about the modules of its crate.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
// what a file says of its modules as the module tree owns it; its look-alike is the notes' borrowed `Note`, which is converted to this once
pub(crate) struct Facts {
    /// Whether the file parses without errors.
    pub clean: Cleanliness,
    /// The modules it declares, in source order.
    pub declared: Vec<ModDeclaration>,
    /// The paths it writes, in source order.
    pub written: Vec<Written>,
}

/// Whether a file parsed without an error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Cleanliness {
    /// It did.
    Clean,
    /// It did not.
    #[default]
    Broken,
}
