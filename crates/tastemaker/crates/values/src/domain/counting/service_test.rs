//! The `counting` service's cases, on values alone.

use std::collections::BTreeMap;

use super::service::counts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::{LoadedWorkspace, WorkspaceSection};
use crate::domain::entries::types::{Amount, Composition, Count};
use crate::domain::findings::types::WorkspaceName;
use crate::domain::paths::types::RepoPath;
use crate::domain::scopes::service::scope_file;
use crate::domain::scopes::types::{
    CompositionRef, CountMessage, CountedEntry, Grouping, Identity, ScopeIndex,
};
use crate::domain::selectors::types::Selectors;

/// How a spec entry was applied: whether it counts its directories whole or per group.
type Input = Grouping;
/// The rules of the counts left waiting, joined, and how many pairs and findings there are.
type Value = String;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_spec_entry_counted_whole_waits_with_its_directories"),
        word: Word::Succeeds,
        input: || Grouping::Whole,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_spec_entry_counted_per_group_is_counted_by_its_groups"),
        word: Word::Succeeds,
        input: || Grouping::Grouped,
        expect: Expect::Value(String::new()),
    },
];

fn workspace() -> LoadedWorkspace {
    LoadedWorkspace {
        file: RepoPath("tastemaker.toml".into()),
        directory: RepoPath::default(),
        section: WorkspaceSection {
            name: WorkspaceName("w".into()),
            rules_exclude: Selectors::default(),
            members: Selectors::default(),
            dependencies: Vec::new(),
        },
        composition: Composition::default(),
        locals: Vec::new(),
        packages: BTreeMap::new(),
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: Vec::new(),
        local_packages: Vec::new(),
    }
}

/// What the counts come to, against what the row's case expects: empty when they agree.
fn subject(grouped: Input) -> Value {
    let root = scope_file(
        RepoPath::default(),
        Identity("tastemaker.toml".into()),
        RepoPath("tastemaker.toml".into()),
        CompositionRef::Root,
        &Composition::default(),
    );
    let application = CountedEntry {
        parent: ScopeIndex(0),
        identity: Identity("p/part".into()),
        count: Count::Exactly(Amount(1)),
        counted: Amount(0),
        grouped,
    };
    let counted = counts(&[root], &[], &[], &[application], &workspace());
    let said = counted
        .checks
        .iter()
        .map(|check| match &check.message {
            CountMessage::Spec { identity } => format!(
                "{} {}",
                crate::domain::findings::service::printed(&check.rule).0,
                identity.0
            ),
            CountMessage::Template { .. } | CountMessage::Grouped { .. } => {
                crate::domain::findings::service::printed(&check.rule)
                    .0
                    .to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let expected = match grouped {
        Grouping::Whole => "p/part#count p/part",
        Grouping::Grouped => "",
    };
    let said = format!("{said} {} {}", counted.pairs.len(), counted.findings.len());
    let expected = format!("{expected} 0 0");
    if said == expected {
        String::new()
    } else {
        said
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
