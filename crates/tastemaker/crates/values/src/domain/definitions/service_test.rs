//! What the `definitions` service finds of a file's folded keys and its local files.

use std::collections::BTreeMap;

use super::service::{folded, governed, locals};
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
use crate::domain::selectors::types::Coverage;
use crate::domain::selectors::types::Selectors;

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// `layers` and `acyclic` written are two warnings.
    Folded,
    /// A local file under a member is found, one elsewhere is not.
    Locals,
    /// Only a nested workspace's root file prunes its tree: a `tastemaker.toml` that declares no workspace prunes nothing.
    Governed,
    /// A nested `tastemaker.toml` that does not read stops the walk with its error.
    Unread,
}

/// What a row asks.
type Input = Question;
/// What the service answers.
type Value = String;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_folded_key_written_is_a_warning"),
        word: Word::Succeeds,
        input: || Question::Folded,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_local_file_is_one_under_a_member"),
        word: Word::Succeeds,
        input: || Question::Locals,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("only_a_nested_workspace_root_prunes_its_tree"),
        word: Word::Succeeds,
        input: || Question::Governed,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_nested_file_that_does_not_read_stops_the_walk"),
        word: Word::Succeeds,
        input: || Question::Unread,
        expect: Expect::Value(String::new()),
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

fn subject(question: Input) -> Value {
    let (said, expected) = match question {
        Question::Folded => (
            folded(
                &workspace("p").composition,
                &RepoPath("tastemaker.toml".into()),
            )
            .len()
            .to_string(),
            "2",
        ),
        Question::Locals => {
            let files = [
                RepoPath("tastemaker.toml".into()),
                RepoPath("crates/a/tastemaker.toml".into()),
                RepoPath("docs/tastemaker.toml".into()),
            ];
            let found = locals(
                &files,
                &RepoPath("tastemaker.toml".into()),
                |directory| {
                    if directory.0.starts_with("crates/") {
                        Coverage::Covered
                    } else {
                        Coverage::Uncovered
                    }
                },
                |_| Coverage::Uncovered,
            );
            (
                found
                    .iter()
                    .map(|path| &*path.0)
                    .collect::<Vec<_>>()
                    .join(","),
                "crates/a/tastemaker.toml",
            )
        }
        Question::Governed => {
            let files = [
                "tastemaker.toml",
                "examples/api/tastemaker.toml",
                "examples/api/src/a.py",
                "tests/fixtures/local/tastemaker.toml",
                "tests/fixtures/local/a.txt",
            ]
            .map(|file| RepoPath(file.into()))
            .to_vec();
            let kept = governed(
                files,
                &RepoPath("tastemaker.toml".into()),
                |_| Coverage::Uncovered,
                |file| -> Result<_, ()> {
                    if file.0.starts_with("examples/") {
                        Ok(crate::domain::definitions::types::Role::Root(
                            WorkspaceSection {
                                name: WorkspaceName("nested".into()),
                                rules_exclude: Selectors::default(),
                                members: Selectors::default(),
                                dependencies: Vec::new(),
                            },
                        ))
                    } else {
                        Ok(crate::domain::definitions::types::Role::Local)
                    }
                },
            )
            .unwrap_or_default();
            (
                kept.iter()
                    .map(|path| &*path.0)
                    .collect::<Vec<_>>()
                    .join(","),
                "tastemaker.toml,tests/fixtures/local/tastemaker.toml,tests/fixtures/local/a.txt",
            )
        }
        Question::Unread => {
            let files = ["tastemaker.toml", "broken/tastemaker.toml", "broken/a.txt"]
                .map(|file| RepoPath(file.into()))
                .to_vec();
            let walked = governed(
                files,
                &RepoPath("tastemaker.toml".into()),
                |_| Coverage::Uncovered,
                |file| Err::<crate::domain::definitions::types::Role, _>(file.0.to_string()),
            );
            (format!("{walked:?}"), "Err(\"broken/tastemaker.toml\")")
        }
    };
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
