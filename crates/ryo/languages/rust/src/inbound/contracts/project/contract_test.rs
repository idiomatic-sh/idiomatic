// one row for each scenario of the project contract, each with its full rows
//! What the `project` contract promises: the manifests of a repository become the rows of
//! `package_dependency` and `workspace_member`, from what the files hold; no manifest, no rows;
//! a manifest that cannot be read is refused and gives no row.

use super::contract::ProjectContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::consulted_test::{members_at, workspace_with};
use crate::fixtures::project::{
    A_ALONE, A_DEPENDS_ON_B, A_DEPENDS_ON_PATCHED_B, REFUSED, TWO_MEMBERS, Wrote, held_manifest,
    linked, manifest, nothing, patched, request_of, use_case, workspace,
};
use crate::fixtures::reading::SHOWN;
use crate::outbound::conditions::factory::cargo_platform;
use crate::outbound::job_scheduling::fixtures::fake::JobSchedulingFake;
use crate::outbound::manifest::factory::toml;
use crate::outbound::pattern_matching::factory::globset;
use crate::outbound::repository::interface::RepositoryPort;
use ryo::languages::contract::{Reading, written};
use ryo::languages::contract::{want_file, want_name, want_span};
use ryo::languages::{self, RootPath};
use std::sync::Arc;

/// What a row gives the contract: what its port would report, and the manifests of the repository.
#[derive(Clone, Copy)]
enum Scenario {
    /// A repository with no manifest.
    Nothing,
    /// A workspace of two members, one depending on the other.
    Workspace,
    /// A workspace of two members, one depending on the other by a version the workspace's `[patch]` puts in its directory.
    Patched,
    /// A manifest that cannot be read.
    Refused,
    /// Manifests of another language only.
    OtherManifests,
    /// A workspace of one package whose files link to one another, and the relations that say so are asked for.
    Linked,
    /// The same, and they are not asked for.
    LinkedButNotAsked,
    /// A manifest the run supplies that is not on disk.
    SuppliedNotOnDisk,
    /// A manifest the run supplies whose text differs from the one on disk, which cannot be read.
    SuppliedDiffersFromDisk,
    /// One contract, built once, serving the repositories of two roots: the root is request data, not configuration of the contract.
    TwoRoots,
}

/// What a row of the cases is given.
type Input = Scenario;

/// What the contract wrote, relation by relation, for the relations asked of it.
type Value = &'static [Wrote];
type Failure = languages::ProjectError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("language_without_manifests_gives_an_empty_report"),
        word: Word::Succeeds,
        input: || Scenario::Nothing,
        expect: Expect::Value(&[
            nothing(languages::Relation::PackageDependency),
            nothing(languages::Relation::WorkspaceMember),
        ]),
    },
    Case {
        name: CaseName("manifests_of_another_language_are_not_asked_about"),
        word: Word::Succeeds,
        input: || Scenario::OtherManifests,
        expect: Expect::Value(&[
            nothing(languages::Relation::PackageDependency),
            nothing(languages::Relation::WorkspaceMember),
        ]),
    },
    Case {
        name: CaseName("a_workspace_gives_a_row_for_each_member_and_each_dependency"),
        word: Word::Succeeds,
        input: || Scenario::Workspace,
        expect: Expect::Value(&[A_DEPENDS_ON_B, TWO_MEMBERS]),
    },
    Case {
        name: CaseName("a_dependency_by_version_the_workspace_patches_to_a_member_is_a_row"),
        word: Word::Succeeds,
        input: || Scenario::Patched,
        expect: Expect::Value(&[A_DEPENDS_ON_PATCHED_B, TWO_MEMBERS]),
    },
    Case {
        name: CaseName(
            "the_files_of_a_package_that_link_give_a_dependency_and_a_row_for_each_file_examined",
        ),
        word: Word::Succeeds,
        input: || Scenario::Linked,
        expect: Expect::Value(&[
            A_DEPENDS_ON_B,
            TWO_MEMBERS,
            Wrote {
                relation: languages::Relation::SourceDependency,
                reading: Reading {
                    rows: &[&[
                        want_file("a/src/b.rs"),
                        want_file("a/src/c.rs"),
                        want_span((1, 5), (1, 21)),
                        want_name("crate::c::thing"),
                    ]],
                    causes: &[],
                },
            },
            Wrote {
                relation: languages::Relation::AnalyzedSource,
                reading: Reading {
                    rows: &[
                        &[want_file("a/src/b.rs")],
                        &[want_file("a/src/c.rs")],
                        &[want_file("a/src/lib.rs")],
                    ],
                    causes: &[],
                },
            },
        ]),
    },
    Case {
        name: CaseName("the_files_are_not_read_when_no_relation_that_links_them_is_asked_for"),
        word: Word::Succeeds,
        input: || Scenario::LinkedButNotAsked,
        expect: Expect::Value(&[A_DEPENDS_ON_B, TWO_MEMBERS]),
    },
    Case {
        name: CaseName("a_manifest_the_run_supplies_is_read_though_it_is_not_on_disk"),
        word: Word::Succeeds,
        input: || Scenario::SuppliedNotOnDisk,
        expect: Expect::Value(&[nothing(languages::Relation::PackageDependency), A_ALONE]),
    },
    Case {
        name: CaseName("the_text_the_run_supplies_is_read_in_place_of_the_text_on_disk"),
        word: Word::Succeeds,
        input: || Scenario::SuppliedDiffersFromDisk,
        expect: Expect::Value(&[nothing(languages::Relation::PackageDependency), A_ALONE]),
    },
    Case {
        name: CaseName("one_contract_serves_the_repositories_of_two_roots"),
        word: Word::Succeeds,
        input: || Scenario::TwoRoots,
        expect: Expect::Value(&[
            Wrote {
                relation: languages::Relation::WorkspaceMember,
                reading: Reading {
                    rows: &[&[want_file("Cargo.toml"), want_file("m/Cargo.toml")]],
                    causes: &[],
                },
            },
            Wrote {
                relation: languages::Relation::WorkspaceMember,
                reading: Reading {
                    rows: &[&[want_file("Cargo.toml"), want_file("n/Cargo.toml")]],
                    causes: &[],
                },
            },
        ]),
    },
    Case {
        name: CaseName("a_manifest_that_cannot_be_read_is_refused_and_gives_no_row"),
        word: Word::Succeeds,
        input: || Scenario::Refused,
        expect: Expect::Value(REFUSED),
    },
];

/// Two workspaces read by one contract: each answers its own members.
fn two_roots() -> Value {
    let contract = ProjectContract::new(
        Arc::new(toml()),
        Arc::new(|root: &RootPath| -> Box<dyn RepositoryPort> {
            Box::new(crate::outbound::repository::factory::native(root.clone()))
        }),
        Arc::new(globset()),
        Arc::new(JobSchedulingFake),
        Arc::new(cargo_platform()),
        Arc::new(crate::outbound::notes::factory::postcard()),
    );
    let (first, second) = (workspace_with("m"), workspace_with("n"));
    Box::leak(Box::new([
        Wrote {
            relation: languages::Relation::WorkspaceMember,
            reading: members_at(&contract, first.path(), "m"),
        },
        Wrote {
            relation: languages::Relation::WorkspaceMember,
            reading: members_at(&contract, second.path(), "n"),
        },
    ]))
}

fn subject(given: Input) -> Result<Value, Failure> {
    if matches!(given, Scenario::TwoRoots) {
        return Ok(two_roots());
    }
    let (mut notes, mut files) = (Vec::new(), Vec::new());
    let mut wanted = vec![
        languages::Relation::PackageDependency,
        languages::Relation::WorkspaceMember,
    ];
    let (held, manifests) = match given {
        Scenario::Linked | Scenario::LinkedButNotAsked => {
            let (made, listed) = linked();
            notes = made;
            files = listed;
            if matches!(given, Scenario::Linked) {
                wanted.extend([
                    languages::Relation::SourceDependency,
                    languages::Relation::AnalyzedSource,
                ]);
            }
            (
                workspace(),
                vec![held_manifest(&workspace(), "a/Cargo.toml")],
            )
        }
        Scenario::Nothing | Scenario::TwoRoots => (Vec::new(), Vec::new()),
        Scenario::OtherManifests => (
            workspace(),
            vec![
                manifest("package.json", "{}"),
                manifest("pyproject.toml", ""),
            ],
        ),
        Scenario::Workspace => (
            workspace(),
            ["Cargo.toml", "a/Cargo.toml", "b/Cargo.toml"]
                .map(|path| held_manifest(&workspace(), path))
                .to_vec(),
        ),
        Scenario::Patched => (
            patched(),
            ["Cargo.toml", "a/Cargo.toml", "b/Cargo.toml"]
                .map(|path| held_manifest(&patched(), path))
                .to_vec(),
        ),
        Scenario::Refused => {
            let broken = vec![("Cargo.toml", "[package\nbroken")];
            let supplied = vec![held_manifest(&broken, "Cargo.toml")];
            (broken, supplied)
        }
        Scenario::SuppliedNotOnDisk => (
            Vec::new(),
            vec![manifest("a/Cargo.toml", "[package]\nname = \"a\"\n")],
        ),
        Scenario::SuppliedDiffersFromDisk => (
            vec![("a/Cargo.toml", "[package\nbroken")],
            vec![manifest("a/Cargo.toml", "[package]\nname = \"a\"\n")],
        ),
    };
    // A manifest the run supplies is a file the run lists, in the order of the paths.
    if files.is_empty() {
        files = manifests.iter().map(|each| each.path.clone()).collect();
        files.sort();
    }
    let contract = use_case(held);
    let listed: Vec<String> = files.iter().map(|each| each.0.to_string()).collect();
    let mut wrote = Vec::new();
    for relation in wanted.clone() {
        let (mut out, mut symbols) = (
            languages::contract::Collect::default(),
            languages::Symbols::default(),
        );
        let resolved = languages::contract::read_project(
            &contract,
            &request_of(&manifests, &notes, &files, relation),
            &mut symbols,
            &mut out,
        )?;
        // The rows number a manifest the run does not list after the last file it does, in the order the run answers them.
        let numbered: Vec<&str> = listed
            .iter()
            .map(String::as_str)
            .chain(resolved.unlisted.iter().map(|each| &*each.0))
            .collect();
        wrote.push(Wrote {
            relation,
            reading: written(
                out,
                symbols,
                (&numbered, &[b'x'; 32]),
                (languages::SourcePath(SHOWN), relation),
                &[],
            ),
        });
    }
    Ok(Box::leak(wrote.into_boxed_slice()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
