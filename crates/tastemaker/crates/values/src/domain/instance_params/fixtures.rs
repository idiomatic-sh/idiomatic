//! The test support of the `instance_params` concept: the spec `p/language` with its one param `lang`, an entry that
//! applies it, and its params at `languages/toml`.

use std::collections::BTreeMap;

use crate::domain::captures::types::Captures;
use crate::domain::definitions::types::{SpecFile, Why};
use crate::domain::entries::types::{
    Alias, Amount, Composition, Count, Described, Description, EntryHeader, EntryIdentity, Imports,
    ParamDeclaration, ParamDeclarations, ParamName, ParamSource, ParamType, ParamValue,
    ParamValues, SpecEntry, ValueWriter,
};
use crate::domain::findings::types::WorkspaceName;
use crate::domain::instance_params::types::{Instance, InstanceParams};
use crate::domain::packages::types::{DefinitionName, Ownership, PackageName, VersionText};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::service::scope_file;
use crate::domain::scopes::types::{CompositionRef, Identity, Scope};
use crate::domain::selectors::types::Selectors;

/// The spec `p/language`, which declares `lang`, taken from `source` when nothing passes it.
#[must_use]
pub fn spec(source: ParamSource) -> SpecFile {
    let mut declared = BTreeMap::new();
    drop(declared.insert(
        ParamName("lang".into()),
        ParamDeclaration {
            kind: ParamType("identifier".into()),
            description: Described::Unsaid,
            source,
        },
    ));
    SpecFile {
        description: Description("d".into()),
        why: Why("w".into()),
        params: ParamDeclarations(declared),
        rules_exclude: Selectors::default(),
        composition: Composition::default(),
        scenarios: Vec::new(),
    }
}

/// An entry of `p/language` passing `passed`.
#[must_use]
pub fn entry(passed: BTreeMap<ParamName, ParamValue>) -> SpecEntry {
    SpecEntry {
        header: EntryHeader {
            package: PackageName("p".into()),
            version: VersionText("0.1.0".into()),
            ownership: Ownership::Shared,
            alias: Alias::Unnamed,
            description: Described::Unsaid,
            include: Selectors::default(),
            exclude: Selectors::default(),
            params: ParamValues(passed),
        },
        spec: DefinitionName("language".into()),
        count: Count::Exactly(Amount(1)),
        imports: Imports::Unrestricted,
    }
}

/// Where `p/language` is placed: its entry and spec, the instance `languages/toml` and the scope that writes the entry.
#[derive(Debug)]
pub struct Placed {
    /// The entry's identity.
    pub identity: EntryIdentity,
    /// The entry.
    pub entry: SpecEntry,
    /// The spec.
    pub spec: SpecFile,
    /// The file the entry is written in.
    pub anchor: RepoPath,
    /// The instance's directory.
    pub directory: RepoPath,
    /// The scope the entry is written in.
    pub parent: Scope,
    /// What the entry's `include` captured.
    pub captures: Captures,
    /// The workspace's name.
    pub workspace: WorkspaceName,
}

/// `p/language` placed at `languages/toml` by an entry that passes `given`, its spec taking `lang` from `source`.
#[must_use]
pub fn placed(given: BTreeMap<ParamName, ParamValue>, source: ParamSource) -> Placed {
    let anchor = RepoPath("tastemaker.toml".into());
    Placed {
        identity: EntryIdentity("p/language".into()),
        entry: entry(given),
        spec: spec(source),
        parent: scope_file(
            RepoPath::default(),
            Identity("tastemaker.toml".into()),
            anchor.clone(),
            CompositionRef::Root,
            &Composition::default(),
        ),
        anchor,
        directory: RepoPath("languages/toml".into()),
        captures: Captures::default(),
        workspace: WorkspaceName("ws".into()),
    }
}

/// The instance `placed` is, each value written by `written`.
#[must_use]
pub fn instance<'a>(placed: &'a Placed, written: &'a dyn ValueWriter) -> Instance<'a> {
    Instance {
        identity: &placed.identity,
        entry: &placed.entry,
        spec: &placed.spec,
        anchor: &placed.anchor,
        directory: &placed.directory,
        parent: &placed.parent,
        captures: &placed.captures,
        workspace: &placed.workspace,
        written,
    }
}

/// The value `lang` is given, and the messages of the findings, joined.
#[must_use]
pub fn said(found: &InstanceParams) -> (String, String) {
    let lang = found
        .values
        .0
        .values()
        .next()
        .map(|captured| captured.0.to_string())
        .unwrap_or_default();
    let messages = found
        .findings
        .iter()
        .map(|finding| finding.message.0.to_string())
        .collect::<Vec<_>>()
        .join("|");
    (lang, messages)
}
