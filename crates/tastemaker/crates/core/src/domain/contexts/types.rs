//! The types of the `contexts` concept: plain data, public fields and derives, with no constructor and no inherent impl.
//! What a template's content sees: its params and captures, and three maps of the instance it is placed in.

use std::collections::BTreeMap;

/// A name a template's content reads.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextKey(pub Box<str>);

/// The names a template's content may read, whatever its control flow.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContextKeys(pub std::collections::BTreeSet<ContextKey>);

/// A value a template's content reads, as text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContextText(pub Box<str>);

/// Whether an entry's file exists for the file's captures, as `present.<entry>` reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Present {
    /// It does.
    Yes,
    /// It does not.
    No,
}

/// One instance of a spec beneath the file's own, as `instances.<spec>` lists it: its `name`, its `path` from the
/// file's instance, and its params and captures.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InstanceValues(pub BTreeMap<ContextKey, ContextText>);

/// What a template's content sees.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RenderContext {
    /// Its params and the captures of its path.
    pub values: BTreeMap<ContextKey, ContextText>,
    /// `captures.<name>`: the sorted values a capture takes beneath the file's own captures.
    pub captured: BTreeMap<ContextKey, Vec<ContextText>>,
    /// `present.<entry>`.
    pub present: BTreeMap<ContextKey, Present>,
    /// `instances.<spec>`.
    pub instances: BTreeMap<ContextKey, Vec<InstanceValues>>,
}

/// Where a file a template places is, and what places it: what `context` builds the template's view from.
#[derive(Clone, Copy, Debug)]
pub struct ContextRequest<'a> {
    /// The workspace and its definitions.
    pub workspace: &'a crate::domain::definitions::types::LoadedWorkspace,
    /// Every scope the placement made.
    pub scopes: &'a [crate::domain::scopes::types::Scope],
    /// What each template entry of the scopes matched.
    pub matched: &'a [crate::domain::scopes::types::Matched],
    /// The scope that places the file.
    pub index: crate::domain::scopes::types::ScopeIndex,
    /// The template entry that places it.
    pub entry: &'a crate::domain::entries::types::TemplateEntry,
    /// The template the entry names.
    pub template: &'a crate::domain::definitions::types::TemplateFile,
    /// The file.
    pub file: &'a crate::domain::paths::types::RepoPath,
    /// What the entry's pattern captured of the file's path.
    pub captures: &'a crate::domain::captures::types::Captures,
    /// How a param's value is written as the content reads it.
    pub written: &'a dyn crate::domain::entries::types::ValueWriter,
}

/// What surrounds a placed file: the placement it is in and the scope that places it, which `surroundings` reads the placed
/// siblings and the nested instances from.
#[derive(Clone, Copy, Debug)]
pub struct Surroundings<'a> {
    /// The workspace and its definitions.
    pub workspace: &'a crate::domain::definitions::types::LoadedWorkspace,
    /// Every scope the placement made.
    pub scopes: &'a [crate::domain::scopes::types::Scope],
    /// What each template entry of the scopes matched.
    pub matched: &'a [crate::domain::scopes::types::Matched],
    /// The scope that places the file.
    pub index: crate::domain::scopes::types::ScopeIndex,
    /// What the entry's pattern captured of the file's path.
    pub captures: &'a crate::domain::captures::types::Captures,
}

/// Why a template's view has no value for one of its params, in a sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unvalued(pub String);

impl From<&str> for ContextKey {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl From<&str> for ContextText {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}
