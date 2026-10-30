// ignore: idiomatic.arch/file-length - one row for each scenario of the project contract, each with its full rows
//! What the `project` contract promises: the manifests of a repository become the rows of
//! `package_dependency` and `workspace_member`, from what the files hold; no manifest, no rows;
//! a manifest that cannot be read is refused and gives no row.

use std::sync::Arc;

use super::contract::ProjectContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::service::note_of;
use crate::domain::module_tree::types::{
    Body, Cleanliness, Facts, ModDeclaration, Segment, Spelling, Written,
};
use crate::domain::paths::types::Section;
use crate::fixtures::reading::SHOWN;
use crate::outbound::conditions::factory::cargo_platform;
use crate::outbound::job_scheduling::fixtures::fake::JobSchedulingFake;
use crate::outbound::manifest::factory::toml;
use crate::outbound::notes::interface::NotesPort as _;
use crate::outbound::pattern_matching::factory::globset;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::repository::interface::RepositoryPort;
use ryo::languages::contract::{Reading, written};
use ryo::languages::contract::{want_file, want_flag, want_name, want_span};
use ryo::languages::{self, RootPath};

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
/// What the contract wrote for one relation: its rows in full, and the causes for which it read the relation only in part.
#[derive(Debug, PartialEq)]
struct Wrote {
    /// The relation.
    relation: languages::Relation,
    /// Its rows and causes.
    reading: Reading,
}

/// What the contract wrote, relation by relation, for the relations asked of it.
type Value = &'static [Wrote];
type Failure = languages::ProjectError;

/// The files of a workspace of two members, `a` depending on `b`.
fn workspace() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
        (
            "a/Cargo.toml",
            "[package]\nname = \"a\"\n[dependencies]\nb = { path = \"../b\" }\n",
        ),
        ("b/Cargo.toml", "[package]\nname = \"b\"\n"),
    ]
}

/// The files of a workspace of two members, `a` depending on `b` by version, which the workspace patches to `b`'s directory.
fn patched() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"a\", \"b\"]\n[patch.crates-io]\nb = { path = \"b\" }\n",
        ),
        (
            "a/Cargo.toml",
            "[package]\nname = \"a\"\n[dependencies]\nb = \"0.1\"\n",
        ),
        ("b/Cargo.toml", "[package]\nname = \"b\"\n"),
    ]
}

/// The notes of `facts`, written as the parse writes them.
fn encode(facts: &Facts) -> languages::Bytes {
    let mut out = languages::Bytes::default();
    crate::outbound::notes::factory::postcard().write(&note_of(facts), &mut out);
    out
}

/// The package `a`, rooted at `a/src/lib.rs`, which declares `b` and `c`; `b` writes a path into `c`. The notes are of the files, in their order; the manifests of the workspace are files of the run too.
fn linked() -> (Vec<languages::Bytes>, Vec<languages::FilePath>) {
    let declared = |name: &str| ModDeclaration {
        around: Vec::new(),
        name: Segment(name.into()),
        body: Body::File,
        path: Section::Absent,
    };
    let notes = vec![
        languages::Bytes::default(),
        languages::Bytes::default(),
        encode(&Facts {
            clean: Cleanliness::Clean,
            declared: Vec::new(),
            written: vec![Written {
                segments: vec![
                    Segment("crate".into()),
                    Segment("c".into()),
                    Segment("thing".into()),
                ],
                enclosing: Vec::new(),
                start: languages::Offset(4),
                end: languages::Offset(20),
                text: Spelling("crate::c::thing".into()),
            }],
        }),
        encode(&Facts {
            clean: Cleanliness::Clean,
            ..Facts::default()
        }),
        encode(&Facts {
            clean: Cleanliness::Clean,
            declared: vec![declared("b"), declared("c")],
            written: Vec::new(),
        }),
        languages::Bytes::default(),
    ];
    let files = [
        "Cargo.toml",
        "a/Cargo.toml",
        "a/src/b.rs",
        "a/src/c.rs",
        "a/src/lib.rs",
        "b/Cargo.toml",
    ]
    .map(|path| languages::FilePath(path.into()))
    .to_vec();
    (notes, files)
}

/// The manifest at `path` as the run supplies it, with `text`.
fn manifest(path: &str, text: &str) -> languages::ManifestFile {
    languages::ManifestFile {
        path: languages::FilePath(path.into()),
        text: languages::ManifestText(text.into()),
    }
}

/// The manifest at `path` of the files `held`, with the text it has there.
fn held_manifest(held: &[(&'static str, &'static str)], path: &str) -> languages::ManifestFile {
    let text = held
        .iter()
        .find(|(each, _)| *each == path)
        .map_or("", |(_, text)| *text);
    manifest(path, text)
}

/// A relation that was read and has no row and no cause.
const fn nothing(relation: languages::Relation) -> Wrote {
    Wrote {
        relation,
        reading: Reading {
            rows: &[],
            causes: &[],
        },
    }
}

/// `package_dependency` for the workspace of two members, `a` depending on `b`.
const A_DEPENDS_ON_B: Wrote = Wrote {
    relation: languages::Relation::PackageDependency,
    reading: Reading {
        rows: &[&[
            want_file("a/Cargo.toml"),
            want_file("b/Cargo.toml"),
            want_name("b"),
            want_name("b"),
            want_name("normal"),
            want_name(""),
            want_flag(false),
            want_name("path"),
            want_name(""),
            want_flag(false),
            want_name(""),
            want_name(""),
            want_name(""),
            want_name(""),
            want_name(""),
            want_name(""),
        ]],
        causes: &[],
    },
};

/// `package_dependency` for the workspace of two members, `a` depending by version on `b`, which the workspace's `[patch]` puts in
/// its directory: the package comes from the registry as the manifest writes it, and the patch's directory is what replaces it.
const A_DEPENDS_ON_PATCHED_B: Wrote = Wrote {
    relation: languages::Relation::PackageDependency,
    reading: Reading {
        rows: &[&[
            want_file("a/Cargo.toml"),
            want_file("b/Cargo.toml"),
            want_name("b"),
            want_name("b"),
            want_name("normal"),
            want_name(""),
            want_flag(false),
            want_name("registry"),
            want_name("0.1"),
            want_flag(false),
            want_name(""),
            want_name(""),
            want_name(""),
            want_name(""),
            want_name(""),
            want_name("b"),
        ]],
        causes: &[],
    },
};

/// `workspace_member` for the workspace of two members.
const TWO_MEMBERS: Wrote = Wrote {
    relation: languages::Relation::WorkspaceMember,
    reading: Reading {
        rows: &[
            &[want_file("Cargo.toml"), want_file("a/Cargo.toml")],
            &[want_file("Cargo.toml"), want_file("b/Cargo.toml")],
        ],
        causes: &[],
    },
};

/// `workspace_member` for the package `a` alone.
const A_ALONE: Wrote = Wrote {
    relation: languages::Relation::WorkspaceMember,
    reading: Reading {
        rows: &[&[want_file("a/Cargo.toml"), want_file("a/Cargo.toml")]],
        causes: &[],
    },
};

/// Both manifest relations, read in part because the one manifest was refused.
const REFUSED: &[Wrote] = &[
    Wrote {
        relation: languages::Relation::PackageDependency,
        reading: Reading {
            rows: &[],
            causes: &["`Cargo.toml` was refused: unclosed table, expected `]`"],
        },
    },
    Wrote {
        relation: languages::Relation::WorkspaceMember,
        reading: Reading {
            rows: &[],
            causes: &["`Cargo.toml` was refused: unclosed table, expected `]`"],
        },
    },
];

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

/// The request of a run over the disk that lists `files` and asks for `relation`.
fn request_of(
    manifests: &[languages::ManifestFile],
    notes: &[languages::Bytes],
    files: &[languages::FilePath],
    relation: languages::Relation,
) -> languages::ProjectRequest {
    languages::ProjectRequest {
        repository: languages::Repository::Disk,
        root: RootPath(".".into()),
        manifests: manifests.to_vec(),
        notes: notes.to_vec(),
        files: files.to_vec(),
        wanted: vec![relation],
    }
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

/// The contract over a repository holding these files, read by the real manifest and matching adapters.
fn use_case(held: Vec<(&'static str, &'static str)>) -> ProjectContract {
    let mut files: Vec<(String, String)> = held
        .into_iter()
        .map(|(path, text)| (path.to_owned(), text.to_owned()))
        .collect();
    for source in ["a/src/lib.rs", "a/src/b.rs", "a/src/c.rs"] {
        files.push((source.to_owned(), String::new()));
    }
    ProjectContract::new(
        Arc::new(toml()),
        Arc::new(move |_: &RootPath| -> Box<dyn RepositoryPort> {
            Box::new(RepositoryFake {
                files: files.clone(),
            })
        }),
        Arc::new(globset()),
        Arc::new(JobSchedulingFake),
        Arc::new(cargo_platform()),
        Arc::new(crate::outbound::notes::factory::postcard()),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(CONSULTED, Word::Succeeds, consulted);
}

/// Where the files the contract reads beyond the manifests it is handed lie around a repository on the disk.
#[derive(Clone, Copy)]
enum Around {
    /// A workspace whose member's manifest is not handed, with a Cargo configuration in the root and one in the directory above it.
    Configured,
    /// The same workspace, with no Cargo configuration.
    Unconfigured,
    /// The same workspace, with only the older configuration, in the directory above it.
    Legacy,
}

/// The rows of the files the contract reads beyond the manifests it is handed: each is answered by its path from the root.
const CONSULTED: &[Case<Around, &'static [&'static str], Failure>] = &[
    Case {
        name: CaseName(
            "the_configurations_found_above_and_in_the_repository_and_the_manifests_read_from_the_disk_are_consulted",
        ),
        word: Word::Succeeds,
        input: || Around::Configured,
        expect: Expect::Value(&[
            "../.cargo/config.toml",
            ".cargo/config.toml",
            "inside/Cargo.toml",
        ]),
    },
    Case {
        name: CaseName(
            "with_no_configuration_the_places_looked_in_and_the_manifests_read_from_the_disk_are_consulted",
        ),
        word: Word::Succeeds,
        input: || Around::Unconfigured,
        expect: Expect::Value(&[
            "../.cargo/config",
            "../.cargo/config.toml",
            ".cargo/config",
            ".cargo/config.toml",
            "inside/Cargo.toml",
        ]),
    },
    Case {
        name: CaseName(
            "with_only_the_older_configuration_the_newer_name_before_it_is_consulted_too",
        ),
        word: Word::Succeeds,
        input: || Around::Legacy,
        expect: Expect::Value(&[
            "../.cargo/config",
            "../.cargo/config.toml",
            ".cargo/config",
            ".cargo/config.toml",
            "inside/Cargo.toml",
        ]),
    },
];

/// The files the contract consulted reading a workspace in a directory beside what `around` says: what lies more than one directory
/// above the root is the machine's, not the case's, and is left out.
fn consulted(around: Around) -> Result<&'static [&'static str], Failure> {
    let above = tempfile::tempdir().expect("a directory");
    let write = |path: &str, text: &str| {
        let at = above.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    };
    let workspace = "[workspace]\nmembers = [\"inside\"]\n";
    write("repository/Cargo.toml", workspace);
    write(
        "repository/inside/Cargo.toml",
        "[package]\nname = \"inside\"\n",
    );
    if matches!(around, Around::Legacy) {
        write(".cargo/config", "[build]\ntarget-dir = \"out\"\n");
    }
    if matches!(around, Around::Configured) {
        write(".cargo/config.toml", "[build]\ntarget-dir = \"out\"\n");
        write("repository/.cargo/config.toml", "[build]\n");
    }
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
    let resolved = languages::contract::read_project(
        &contract,
        &languages::ProjectRequest {
            repository: languages::Repository::Disk,
            root: RootPath(above.path().join("repository").to_string_lossy().into()),
            manifests: vec![manifest("Cargo.toml", workspace)],
            notes: Vec::new(),
            files: vec![languages::FilePath("Cargo.toml".into())],
            wanted: vec![languages::Relation::WorkspaceMember],
        },
        &mut languages::Symbols::default(),
        &mut languages::contract::Collect::default(),
    )?;
    let near: Vec<&'static str> = resolved
        .consulted
        .iter()
        .filter(|each| !each.0.starts_with("../../"))
        .map(|each| &*Box::leak(each.0.clone()))
        .collect();
    Ok(Box::leak(near.into_boxed_slice()))
}

/// A directory holding a workspace whose one member is `member`.
fn workspace_with(member: &str) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    std::fs::write(
        root.path().join("Cargo.toml"),
        format!("[workspace]\nmembers = [\"{member}\"]\n"),
    )
    .expect("a manifest");
    std::fs::create_dir_all(root.path().join(member)).expect("a directory");
    std::fs::write(
        root.path().join(member).join("Cargo.toml"),
        format!("[package]\nname = \"{member}\"\n"),
    )
    .expect("a manifest");
    root
}

/// What one contract, built once with the ports it calls, writes for `workspace_member` of the repository at `root`.
fn members_at(contract: &ProjectContract, root: &std::path::Path, member: &str) -> Reading {
    let (mut out, mut symbols) = (
        languages::contract::Collect::default(),
        languages::Symbols::default(),
    );
    let resolved = languages::contract::read_project(
        contract,
        &languages::ProjectRequest {
            repository: languages::Repository::Disk,
            root: RootPath(root.to_string_lossy().into()),
            manifests: vec![manifest(
                "Cargo.toml",
                &std::fs::read_to_string(root.join("Cargo.toml")).expect("a manifest"),
            )],
            notes: Vec::new(),
            files: vec![
                languages::FilePath("Cargo.toml".into()),
                languages::FilePath(format!("{member}/Cargo.toml").into()),
            ],
            wanted: vec![languages::Relation::WorkspaceMember],
        },
        &mut symbols,
        &mut out,
    )
    .expect("the project is read");
    let shown = format!("{member}/Cargo.toml");
    // The rows number a manifest the run does not list after the two it does.
    let numbered: Vec<&str> = ["Cargo.toml", shown.as_str()]
        .into_iter()
        .chain(resolved.unlisted.iter().map(|each| &*each.0))
        .collect();
    written(
        out,
        symbols,
        (&numbered, b""),
        (
            languages::SourcePath(SHOWN),
            languages::Relation::WorkspaceMember,
        ),
        &[],
    )
}
