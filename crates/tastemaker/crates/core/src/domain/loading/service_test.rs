//! What the `loading` service asks for next, and what it refuses.

use std::collections::BTreeMap;

use super::service::{begin, next};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::definitions::types::{LoadedWorkspace, WorkspaceSection};
use crate::domain::entries::types::{
    Acyclicity, Alias, Amount, Composition, Described, EntryHeader, Ordering, ParamValues,
    RuleEntry,
};
use crate::domain::findings::types::WorkspaceName;
use crate::domain::packages::types::{
    DefinitionName, Ownership, PackageManifest, PackageName, PackageRoot, Reach, ResolvedPackage,
    SourcePath, VersionText,
};
use crate::domain::paths::types::RepoPath;
use crate::domain::selectors::types::Selectors;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The first definition to read is the rule the root names.
    Next,
    /// A rule of a package no dependency resolves is a problem.
    Unresolved,
}

/// What a row asks.
type Input = Question;
/// What the service answers.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_first_definition_to_read_is_the_one_the_root_names"),
        word: Word::Succeeds,
        input: || Question::Next,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_definition_of_an_unresolved_package_is_refused"),
        word: Word::Refuses,
        input: || Question::Unresolved,
        expect: Expect::error(()),
    },
];

fn workspace(package: &str) -> LoadedWorkspace {
    let mut packages = BTreeMap::new();
    drop(packages.insert(
        PackageName("p".into()),
        ResolvedPackage {
            directory: PackageRoot("/p".into()),
            pin: crate::domain::packages::types::PackageSource::Path(SourcePath("p".into())),
            manifest: PackageManifest {
                name: PackageName("p".into()),
                dependencies: Vec::new(),
            },
            reach: Reach::Direct,
        },
    ));
    LoadedWorkspace {
        file: RepoPath("tastemaker.toml".into()),
        directory: RepoPath::default(),
        section: WorkspaceSection {
            name: WorkspaceName("w".into()),
            rules_exclude: Selectors::default(),
            members: Selectors::default(),
            dependencies: Vec::new(),
        },
        composition: Composition {
            rules: vec![RuleEntry {
                header: EntryHeader {
                    package: PackageName(package.into()),
                    version: VersionText("0.1.0".into()),
                    ownership: Ownership::Shared,
                    alias: Alias::Unnamed,
                    description: Described::Unsaid,
                    include: Selectors::default(),
                    exclude: Selectors::default(),
                    params: ParamValues::default(),
                },
                rule: DefinitionName("r".into()),
            }],
            ordering: Ordering {
                layers: Amount(2),
                independent: Amount(0),
                acyclic: Acyclicity::Written,
            },
            ..Composition::default()
        },
        locals: Vec::new(),
        packages,
        specs: BTreeMap::new(),
        rules: BTreeMap::new(),
        templates: BTreeMap::new(),
        latest: BTreeMap::new(),
        files: Vec::new(),
        local_packages: Vec::new(),
    }
}

fn subject(question: Input) -> Result<Value, Failure> {
    let (said, expected) = match question {
        Question::Next => {
            let mut loading = begin(workspace("p"));
            let wanted = next(&mut loading).ok_or(())?;
            (
                format!("{} {}", wanted.directory.0, wanted.file.0),
                "/p rules/r/0.1.0/rule.toml",
            )
        }
        Question::Unresolved => {
            let mut loading = begin(workspace("q"));
            next(&mut loading).map(drop).ok_or(())?;
            (String::new(), "")
        }
    };
    Ok(if said == expected {
        String::new()
    } else {
        said
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
