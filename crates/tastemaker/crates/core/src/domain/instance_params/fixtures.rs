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
use crate::domain::instance_params::service::params;
use crate::domain::instance_params::types::Instance;
use crate::domain::packages::types::{DefinitionName, Ownership, PackageName, VersionText};
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::service::scope_file;
use crate::domain::scopes::types::{CompositionRef, Identity};
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

/// The value `lang` is given at `languages/toml` when the entry passes `given` and the spec takes it from `source`, each
/// value written by `written`, and the messages of the findings, joined.
#[must_use]
pub fn found(
    given: BTreeMap<ParamName, ParamValue>,
    source: ParamSource,
    written: &dyn ValueWriter,
) -> (String, String) {
    let spec = spec(source);
    let entry = entry(given);
    let anchor = RepoPath("tastemaker.toml".into());
    let directory = RepoPath("languages/toml".into());
    let parent = scope_file(
        RepoPath::default(),
        Identity("tastemaker.toml".into()),
        anchor.clone(),
        CompositionRef::Root,
        &Composition::default(),
    );
    let found = params(Instance {
        identity: &EntryIdentity("p/language".into()),
        entry: &entry,
        spec: &spec,
        anchor: &anchor,
        directory: &directory,
        parent: &parent,
        captures: &Captures::default(),
        workspace: &crate::domain::findings::types::WorkspaceName("ws".into()),
        written,
    });
    let lang = found
        .values
        .0
        .values()
        .next()
        .map(|captured| captured.0.to_string())
        .unwrap_or_default();
    let said = found
        .findings
        .iter()
        .map(|finding| finding.message.0.to_string())
        .collect::<Vec<_>>()
        .join("|");
    (lang, said)
}
