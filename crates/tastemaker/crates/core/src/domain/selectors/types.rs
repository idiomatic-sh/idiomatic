//! The types of the `selectors` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! A selector says where: a glob, a regex, the instances of a part, or the files an entry placed.

use crate::domain::captures::types::{CaptureName, CaptureValue, Captures, Interpolations};
use crate::domain::packages::types::DefinitionName;
use crate::domain::paths::types::RepoPath;

/// A path pattern as written: `*` one segment, `**` any depth, `?` one character, `{name}` a capture within a segment,
/// `{*name}` one or more whole segments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathPattern(pub Box<str>);

/// A regular expression over the paths beneath the place a selector is written for.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegexText(pub Box<str>);

/// The name of a variable a selector list or a value refers to.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableName(pub Box<str>);

/// Whether a part selector reaches the instances directly beneath the place only, or at any depth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Recursion {
    /// The instances directly beneath, none of another instance between.
    Direct,
    /// The instances at any depth.
    Recursive,
}

/// The instances of one spec, by the spec's name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartSelector {
    /// The spec whose instances are selected.
    pub part: DefinitionName,
    /// How deep.
    pub recursion: Recursion,
}

/// The files one template entry placed, by the entry's reference: its alias, or its template's name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlacedBySelector {
    /// The entry's reference.
    pub entry: DefinitionName,
}

/// One selector of a list.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Selector {
    /// A path pattern, written as a bare string.
    Glob(PathPattern),
    /// `{ regex = "…" }`.
    Regex(RegexText),
    /// `{ part = "…" }`, with `recursive = true` for any depth.
    Part(PartSelector),
    /// `{ placed-by = "…" }`.
    PlacedBy(PlacedBySelector),
    /// `{ variable = "…" }`: the variable's list spliced in, resolved before placement.
    Variable(VariableName),
}

/// A selector list: `include`, `exclude`, `rules-exclude`, `members`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Selectors(pub Vec<Selector>);

/// A pattern spelled for the matching port: the glob every path passes first, and, when the pattern captures or holds a
/// `?`, the regular expression that binds each capture, in the order of `groups`.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Spelling {
    /// The glob, for `globset`, with `/` a literal separator.
    pub glob: SpelledGlob,
    /// The regular expression, anchored, when the pattern needs one.
    pub binder: Binding,
    /// The capture each group of the expression binds, in order.
    pub groups: Vec<CaptureName>,
}

/// Whether a spelled pattern carries a binding expression.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Binding {
    /// The glob alone decides.
    #[default]
    GlobOnly,
    /// This expression binds the captures and decides `?`.
    Expression(BinderText),
}

/// The glob a pattern is spelled as for the matching port, with `/` a literal separator.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpelledGlob(pub Box<str>);

impl From<&str> for SpelledGlob {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::ops::Deref for SpelledGlob {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SpelledGlob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The anchored regular expression that binds a pattern's captures.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BinderText(pub Box<str>);

impl From<&str> for BinderText {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl std::ops::Deref for BinderText {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

/// A path as a pattern's test reads it: relative to the pattern's base.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PathRest<'a>(pub &'a str);

/// The place of a glob in the set the matching port compiled it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobIndex(pub usize);

/// Whether one path matched, with what its captures took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Match {
    /// It matched.
    Matched(Captures),
    /// It did not.
    Missed,
}

/// Whether a selector list or a pattern covers a path: matches it or one of its ancestors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// It covers the path.
    Covered,
    /// It does not.
    Uncovered,
}

/// A pattern placed at a base directory, compiled: its text, base and literal prefix, its capture names in order, and
/// the compiled tests the matching port made.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PathMatcher {
    /// The pattern as written.
    pub text: PathPattern,
    /// The directory it is placed at.
    pub base: RepoPath,
    /// The longest literal directory it starts with, beneath the base.
    pub prefix: RepoPath,
    /// What it matches: the base itself (`.`), or a compiled test.
    pub shape: MatcherShape,
    /// The values interpolated into it.
    pub values: Interpolations,
    /// The `{name:dir}` captures of its last segment, each of whose values names a directory beside the file.
    pub directories: Vec<CaptureName>,
}

/// What a placed pattern matches.
#[derive(Clone, Debug)]
pub enum MatcherShape {
    /// The base directory itself: the pattern `.`.
    Itself,
    /// The compiled glob and binder, and the capture each group binds.
    Compiled(CompiledPattern),
}

/// A pattern compiled by the matching port.
#[derive(Clone)]
pub struct CompiledPattern {
    /// The compiled test.
    pub test: std::sync::Arc<dyn PathTest>,
    /// The captures, in the order the expression binds them.
    pub groups: Vec<CaptureName>,
    /// Whether one name is bound twice, so its captures can disagree.
    pub repeats: Repetition,
    /// Whether the glob alone decides, so the pattern can be handed on as that glob.
    pub glob: GlobForm,
}

/// Whether a compiled pattern's glob alone decides what it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GlobForm {
    /// This glob, relative to the pattern's base, decides alone.
    Alone(SpelledGlob),
    /// An expression decides too: the pattern is matched only here.
    Bound,
}

/// Whether a capture name occurs twice in one pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Repetition {
    /// Every name is bound once.
    Once,
    /// A name is bound twice.
    Repeated,
}

/// A selector resolved at one place: a compiled pattern, or the set of paths a part or placed-by selector names.
#[derive(Clone, Debug)]
pub enum Resolved {
    /// A glob or regex, compiled.
    Pattern(PathMatcher),
    /// Directories or files named one by one; each covers itself and what lies beneath it.
    Paths(std::collections::BTreeSet<RepoPath>),
}

/// A selector list resolved at one place, in the order written.
#[derive(Clone, Debug, Default)]
pub struct ResolvedSelectors(pub Vec<Resolved>);

/// Globs the matching port compiles together: a path is tested against all of them in one pass.
pub trait PathSet: Send + Sync {
    /// The positions of the globs `rest` passes, written into `into`, which is cleared first.
    fn passing(&self, rest: PathRest<'_>, into: &mut Vec<GlobIndex>);
}

/// The test the matching port compiles a spelled pattern into.
pub trait PathTest: Send + Sync {
    /// Whether `rest`, a path relative to the pattern's base, passes the glob and, when there is one, the expression.
    fn passes(&self, rest: PathRest<'_>) -> Coverage;

    /// The values each group of the expression binds for `rest`, in order; none when it does not match. A pattern with
    /// no expression binds nothing and answers an empty list when the glob passes.
    fn bind(&self, rest: PathRest<'_>) -> Option<Vec<CaptureValue>>;
}

/// Whether a selector kind resolved a selector.
#[derive(Clone, Debug)]
pub enum Resolution {
    /// It did: this is what it selects.
    Resolved(Resolved),
    /// The selector is of another kind.
    NotMine,
}

/// What compiles a spelling into the test the matching port makes: the domain is handed one, and any function of that
/// shape is one.
pub trait PatternCompiler: Sync {
    /// The test `spelling` compiles into.
    ///
    /// # Errors
    ///
    /// When the glob or the expression does not compile.
    fn compile(&self, spelling: &Spelling) -> Result<std::sync::Arc<dyn PathTest>, PatternFailure>;
}

impl<F> PatternCompiler for F
where
    F: Fn(&Spelling) -> Result<std::sync::Arc<dyn PathTest>, PatternFailure> + Sync,
{
    fn compile(&self, spelling: &Spelling) -> Result<std::sync::Arc<dyn PathTest>, PatternFailure> {
        self(spelling)
    }
}

/// What compiles globs together into the set the matching port makes: the domain is handed one, and any function of that
/// shape is one.
pub trait SetCompiler: Sync {
    /// The set `globs` compile into.
    ///
    /// # Errors
    ///
    /// When a glob does not compile.
    fn compile_set(&self, globs: &[SetGlob])
    -> Result<std::sync::Arc<dyn PathSet>, PatternFailure>;
}

impl<F> SetCompiler for F
where
    F: Fn(&[SetGlob]) -> Result<std::sync::Arc<dyn PathSet>, PatternFailure> + Sync,
{
    fn compile_set(
        &self,
        globs: &[SetGlob],
    ) -> Result<std::sync::Arc<dyn PathSet>, PatternFailure> {
        self(globs)
    }
}

/// Why the matching port could not compile a spelling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternFailure(pub Box<str>);

/// One spec instance a part selector can select: its directory and its spec's name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartInstance {
    /// Its directory.
    pub directory: RepoPath,
    /// The spec it is an instance of.
    pub spec: DefinitionName,
}

/// One placed file a placed-by selector can select: its path and the reference of the entry that placed it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlacedPath {
    /// The file.
    pub path: RepoPath,
    /// The entry's reference: its alias, or its template's name.
    pub entry: DefinitionName,
}

/// Where a selector list is resolved: the directory it is written for, the values it interpolates, the instances and
/// placed files known so far, and the matching port's compile.
#[derive(Clone, Copy)]
pub struct SelectionContext<'a> {
    /// The directory paths are relative to.
    pub base: &'a RepoPath,
    /// The values interpolated into `{name}`s.
    pub values: &'a Interpolations,
    /// The spec instances placed so far.
    pub instances: &'a [PartInstance],
    /// The files placed so far.
    pub placed: &'a [PlacedPath],
    /// The matching port's compile.
    pub compile: &'a dyn PatternCompiler,
}

impl std::fmt::Debug for SelectionContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

/// One glob of a set, as written.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SetGlob(pub Box<str>);

impl From<bool> for Coverage {
    fn from(covered: bool) -> Self {
        if covered {
            Self::Covered
        } else {
            Self::Uncovered
        }
    }
}

/// Whether a placed pattern matches a path itself, not one of its ancestors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Matching {
    /// It matches the path.
    Matches,
    /// It does not.
    Misses,
}

impl From<bool> for Matching {
    fn from(matched: bool) -> Self {
        if matched { Self::Matches } else { Self::Misses }
    }
}

/// A path of the repository, borrowed as the text a pattern matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepoPathText<'a>(pub &'a str);

/// The place of a selector in its list, from zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectorIndex(pub usize);

/// The first selector of a list that matches a path itself: its place and what it captured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirstMatch {
    /// Its place in the list.
    pub position: SelectorIndex,
    /// What its captures took.
    pub captures: Captures,
}

impl std::fmt::Debug for CompiledPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
