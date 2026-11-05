//! The `packages` service's cases, over repositories held in memory and read by the real manifest and matching adapters.

use ryo::languages::Optionality;
use std::collections::BTreeMap;

use super::errors::PackagesError;
use super::service::{numbered, read, refused_membership, root_for};
use super::types::Packages;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::domain::paths::types::RepoPath;
use crate::domain::workspaces::errors::WorkspacesError;
use crate::domain::workspaces::fixtures::{Files, answering};
use crate::domain::workspaces::service::load;

/// What a row of the cases asks of a repository.
#[derive(Clone, Copy)]
enum Scenario {
    /// What its manifests declare.
    Read(Files),
    /// The workspace the package at this manifest is in.
    ListIndex(Files, &'static str),
    /// The number of the file of each manifest its manifests name, when the run lists these files.
    Number(Files, &'static [&'static str]),
    /// Whether each manifest refused leaves which workspace holds it unread.
    WorkspacePackageMembership(Files),
}

/// What a row of the cases asks.
type Input = Scenario;
/// What the manifests declare, one line for each workspace, package, target and dependency, then the manifests refused.
type Value = &'static str;
type Failure = PackagesError;

/// A workspace of two members named by a pattern, one excluded, the root a virtual manifest.
const GLOB_MEMBERS: Files = &[
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/old\"]\n",
    ),
    (
        "crates/a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
    ),
    ("crates/a/src/lib.rs", ""),
    (
        "crates/b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\n",
    ),
    ("crates/b/src/main.rs", ""),
    (
        "crates/old/Cargo.toml",
        "[package]\nname = \"old\"\nversion = \"0.1.0\"\n",
    ),
    ("crates/old/src/lib.rs", ""),
];

/// A package below the workspace that no pattern names is a member when a member depends on it by path.
const PATH_MEMBER: Files = &[
    ("Cargo.toml", "[workspace]\nmembers = [\"app\"]\n"),
    (
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[dependencies]\nhelper = { path = \"../helper\", optional = true }\n[target.'cfg( all( unix, target_pointer_width = \"64\" ) )'.dev-dependencies]\nhelper = { path = \"../helper\" }\n",
    ),
    ("app/src/lib.rs", ""),
    (
        "helper/Cargo.toml",
        "[package]\nname = \"helper\"\nversion = \"0.1.0\"\n",
    ),
    ("helper/src/lib.rs", ""),
];

/// A dependency a member inherits from the workspace, under another name.
const INHERITED: Files = &[
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"app\"]\n[workspace.dependencies]\nshared = { path = \"shared\", package = \"shared-real\" }\n",
    ),
    (
        "app/Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n[build-dependencies]\nshared = { workspace = true }\n",
    ),
    (
        "shared/Cargo.toml",
        "[package]\nname = \"shared-real\"\nversion = \"0.1.0\"\n",
    ),
];

/// A package left out of the workspace above it that takes a value from a workspace it is not in.
const STRANDED: Files = &[
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"a\"]\nexclude = [\"b\"]\n",
    ),
    (
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
    ),
    (
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion.workspace = true\n",
    ),
];

/// A package the workspace above it does not hold, in a directory it does not leave out.
const NOT_A_MEMBER: Files = &[
    ("Cargo.toml", "[workspace]\nmembers = [\"a\"]\n"),
    ("a/Cargo.toml", "[package]\nname = \"a\"\n"),
    ("z/Cargo.toml", "[package]\nname = \"z\"\n"),
];

/// A workspace one of whose members has no manifest.
const UNREADABLE: Files = &[
    ("Cargo.toml", "[workspace]\nmembers = [\"a\", \"gone\"]\n"),
    ("a/Cargo.toml", "[package]\nname = \"a\"\n"),
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let (input, candidate) = match asked {
        Scenario::Read(files) => (files, None),
        Scenario::ListIndex(files, candidate) => (files, Some(candidate)),
        Scenario::Number(files, listed) => return Ok(numbers(files, listed)),
        Scenario::WorkspacePackageMembership(files) => return Ok(memberships(files)),
    };
    if let Some(candidate) = candidate {
        let path = RepoPath(candidate.into());
        let root = answering(input, |looking| {
            let held = load(looking, &path).unwrap_or_default();
            root_for(looking, &path, &held)
        })?;
        return Ok(Box::leak(root.0));
    }
    let manifests: Vec<RepoPath> = input
        .iter()
        .filter(|(path, _)| path.ends_with("Cargo.toml"))
        .map(|(path, _)| RepoPath((*path).into()))
        .collect();
    let packages = answering(input, |looking| read(&manifests, looking));
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Ok(Box::leak(shown(&packages).into_boxed_str()))
}

/// The number each manifest the manifests name has when the run lists `listed`, in the order of the numbers, one line for each.
fn numbers(input: Files, listed: &[&str]) -> Value {
    let manifests: Vec<RepoPath> = input
        .iter()
        .filter(|(path, _)| path.ends_with("Cargo.toml"))
        .map(|(path, _)| RepoPath((*path).into()))
        .collect();
    let packages = answering(input, |looking| read(&manifests, looking));
    let files: Vec<ryo::languages::FilePath> = listed
        .iter()
        .map(|path| ryo::languages::FilePath((*path).into()))
        .collect();
    let mut placed: Vec<(u32, String)> = numbered(&files, &packages)
        .into_iter()
        .map(|(manifest, number)| (number.0, manifest.0.to_string()))
        .collect();
    placed.sort();
    let lines: Vec<String> = placed
        .into_iter()
        .map(|(number, manifest)| format!("{manifest}: {number}"))
        .collect();
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(lines.join("\n").into_boxed_str())
}

/// Each manifest refused and whether which workspace holds it was read, one line for each.
fn memberships(input: Files) -> Value {
    let manifests: Vec<RepoPath> = input
        .iter()
        .filter(|(path, _)| path.ends_with("Cargo.toml"))
        .map(|(path, _)| RepoPath((*path).into()))
        .collect();
    let packages = answering(input, |looking| read(&manifests, looking));
    let lines: Vec<String> = packages
        .refused
        .iter()
        .map(|refused| format!("{}: {:?}", refused.manifest.0, refused_membership(refused)))
        .collect();
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(lines.join("\n").into_boxed_str())
}

/// What the manifests declare, shown.
fn shown(packages: &Packages) -> String {
    let mut lines: Vec<String> = Vec::new();
    for member in &packages.members {
        lines.push(format!("{} holds {}", member.workspace.0, member.package.0));
    }
    let roots: BTreeMap<&str, Vec<&str>> = packages
        .roots
        .iter()
        .map(|(directory, targets)| {
            (
                &*directory.0,
                targets.iter().map(|target| &*target.0).collect(),
            )
        })
        .collect();
    for (directory, targets) in roots {
        lines.push(format!("{directory}: {}", targets.join(" ")));
    }
    for each in &packages.dependencies {
        lines.push(format!(
            "{} -> {} {:?} as {} of {} {:?} [{}] {}",
            each.source.0,
            each.target.as_ref().map_or("", |target| &*target.0),
            each.origin,
            each.binding,
            each.package,
            each.kind,
            each.condition,
            (each.optional == Optionality::Optional)
        ));
    }
    for each in &packages.refused {
        lines.push(format!("refused {}: {}", each.manifest.0, each.reason));
    }
    lines.join("\n")
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "members_named_by_a_pattern_are_found_and_an_excluded_one_is_a_workspace_of_its_own",
        ),
        word: Word::Succeeds,
        input: || Scenario::Read(GLOB_MEMBERS),
        expect: Expect::Value(
            "Cargo.toml holds crates/a/Cargo.toml\nCargo.toml holds crates/b/Cargo.toml\ncrates/old/Cargo.toml holds crates/old/Cargo.toml\ncrates/a: crates/a/src/lib.rs\ncrates/b: crates/b/src/main.rs\ncrates/old: crates/old/src/lib.rs",
        ),
    },
    Case {
        name: CaseName("a_manifest_the_run_lists_is_numbered_by_its_place_among_the_files"),
        word: Word::Succeeds,
        input: || {
            Scenario::Number(
                GLOB_MEMBERS,
                &[
                    "Cargo.toml",
                    "crates/a/Cargo.toml",
                    "crates/a/src/lib.rs",
                    "crates/b/Cargo.toml",
                    "crates/old/Cargo.toml",
                ],
            )
        },
        expect: Expect::Value(
            "Cargo.toml: 0\ncrates/a/Cargo.toml: 1\ncrates/b/Cargo.toml: 3\ncrates/old/Cargo.toml: 4",
        ),
    },
    Case {
        name: CaseName("a_manifest_the_run_does_not_list_is_numbered_after_the_files_it_lists"),
        word: Word::Succeeds,
        input: || {
            Scenario::Number(
                GLOB_MEMBERS,
                &[
                    "Cargo.toml",
                    "crates/a/Cargo.toml",
                    "crates/a/src/lib.rs",
                    "crates/b/Cargo.toml",
                ],
            )
        },
        expect: Expect::Value(
            "Cargo.toml: 0\ncrates/a/Cargo.toml: 1\ncrates/b/Cargo.toml: 3\ncrates/old/Cargo.toml: 4",
        ),
    },
    Case {
        name: CaseName(
            "a_package_a_member_depends_on_by_path_is_a_member_and_a_condition_is_shown_as_cargo_shows_it",
        ),
        word: Word::Succeeds,
        input: || Scenario::Read(PATH_MEMBER),
        expect: Expect::Value(
            "Cargo.toml holds app/Cargo.toml\nCargo.toml holds helper/Cargo.toml\napp: app/src/lib.rs\nhelper: helper/src/lib.rs\napp/Cargo.toml -> helper/Cargo.toml Path as helper of helper Normal [] true\napp/Cargo.toml -> helper/Cargo.toml Path as helper of helper Dev [cfg(all(unix, target_pointer_width = \"64\"))] false",
        ),
    },
    Case {
        name: CaseName(
            "a_dependency_declared_by_the_workspace_is_inherited_under_the_name_it_is_bound_to",
        ),
        word: Word::Succeeds,
        input: || Scenario::Read(INHERITED),
        expect: Expect::Value(
            "Cargo.toml holds app/Cargo.toml\nCargo.toml holds shared/Cargo.toml\napp: \nshared: \napp/Cargo.toml -> shared/Cargo.toml Path as shared of shared-real Build [] false",
        ),
    },
    Case {
        name: CaseName("a_package_outside_every_workspace_that_inherits_from_one_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::Read(STRANDED),
        expect: Expect::Value(
            "Cargo.toml holds a/Cargo.toml\na: \nrefused b/Cargo.toml: error inheriting a value from the workspace root manifest: failed to find a workspace root",
        ),
    },
    Case {
        name: CaseName("a_package_a_workspace_does_not_hold_and_does_not_leave_out_is_refused"),
        word: Word::Succeeds,
        input: || Scenario::Read(NOT_A_MEMBER),
        expect: Expect::Value(
            "Cargo.toml holds a/Cargo.toml\na: \nrefused z/Cargo.toml: current package believes it's in a workspace when it's not",
        ),
    },
    Case {
        name: CaseName("a_package_in_no_workspace_is_known_to_be_in_none"),
        word: Word::Succeeds,
        input: || Scenario::WorkspacePackageMembership(NOT_A_MEMBER),
        expect: Expect::Value("z/Cargo.toml: Analyzed"),
    },
    Case {
        name: CaseName("a_package_refused_for_another_reason_leaves_its_workspace_unread"),
        word: Word::Succeeds,
        input: || Scenario::WorkspacePackageMembership(STRANDED),
        expect: Expect::Value("b/Cargo.toml: Unanalyzed"),
    },
    Case {
        name: CaseName("a_package_a_workspace_holds_is_in_that_workspace"),
        word: Word::Succeeds,
        input: || Scenario::ListIndex(GLOB_MEMBERS, "crates/a/Cargo.toml"),
        expect: Expect::Value("Cargo.toml"),
    },
    Case {
        name: CaseName("a_package_a_workspace_does_not_hold_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ListIndex(NOT_A_MEMBER, "z/Cargo.toml"),
        expect: Expect::Error(PackagesError::NotAMember),
    },
    Case {
        name: CaseName("a_package_that_takes_a_value_from_a_workspace_it_is_not_in_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ListIndex(STRANDED, "b/Cargo.toml"),
        expect: Expect::Error(PackagesError::NoWorkspaceToInheritFrom),
    },
    Case {
        name: CaseName("a_workspace_that_cannot_be_read_refuses_the_package_in_it"),
        word: Word::Fails,
        input: || Scenario::ListIndex(UNREADABLE, "a/Cargo.toml"),
        expect: Expect::Refusal(|error| {
            selects(
                matches!(error, PackagesError::Workspaces(WorkspacesError::Member { manifest, reason })
                if &*manifest.0 == "gone/Cargo.toml" && &*reason.0 == "gone/Cargo.toml"),
            )
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}
