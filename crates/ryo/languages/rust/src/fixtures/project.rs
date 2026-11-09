//! The test support of the `project` cases.

use crate::domain::fact_notes::service::note_of;
use crate::domain::module_tree::types::{
    Body, Cleanliness, Facts, ModDeclaration, Segment, Spelling, Written,
};
use crate::domain::paths::types::Section;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::conditions::factory::cargo_platform;
use crate::outbound::job_scheduling::fixtures::fake::JobSchedulingFake;
use crate::outbound::manifest::factory::toml;
use crate::outbound::notes::interface::NotesPort as _;
use crate::outbound::pattern_matching::factory::globset;
use crate::outbound::repository::fixtures::fake::RepositoryFake;
use crate::outbound::repository::interface::RepositoryPort;
use ryo::languages::contract::Reading;
use ryo::languages::contract::{want_file, want_flag, want_name};
use ryo::languages::{self, RootPath};
use std::sync::Arc;

/// What the contract wrote for one relation: its rows in full, and the causes for which it read the relation only in part.
#[derive(Debug, PartialEq)]
pub(crate) struct Wrote {
    /// The relation.
    pub(crate) relation: languages::Relation,
    /// Its rows and causes.
    pub(crate) reading: Reading,
}

/// The files of a workspace of two members, `a` depending on `b`.
pub(crate) fn workspace() -> Vec<(&'static str, &'static str)> {
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
pub(crate) fn patched() -> Vec<(&'static str, &'static str)> {
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
pub(crate) fn encode(facts: &Facts) -> languages::Bytes {
    let mut out = languages::Bytes::default();
    crate::outbound::notes::factory::postcard().write(&note_of(facts), &mut out);
    out
}

/// The package `a`, rooted at `a/src/lib.rs`, which declares `b` and `c`; `b` writes a path into `c`. The notes are of the files, in their order; the manifests of the workspace are files of the run too.
pub(crate) fn linked() -> (Vec<languages::Bytes>, Vec<languages::FilePath>) {
    let declared = |name: &str| ModDeclaration {
        gate: crate::domain::module_tree::types::Gate::Always,
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
                bound: None,
                block: None,
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
pub(crate) fn manifest(path: &str, text: &str) -> languages::ManifestFile {
    languages::ManifestFile {
        path: languages::FilePath(path.into()),
        text: languages::ManifestText(text.into()),
    }
}

/// The manifest at `path` of the files `held`, with the text it has there.
pub(crate) fn held_manifest(
    held: &[(&'static str, &'static str)],
    path: &str,
) -> languages::ManifestFile {
    let text = held
        .iter()
        .find(|(each, _)| *each == path)
        .map_or("", |(_, text)| *text);
    manifest(path, text)
}

/// A relation that was read and has no row and no cause.
pub(crate) const fn nothing(relation: languages::Relation) -> Wrote {
    Wrote {
        relation,
        reading: Reading {
            rows: &[],
            causes: &[],
        },
    }
}

/// `package_dependency` for the workspace of two members, `a` depending on `b`.
pub(crate) const A_DEPENDS_ON_B: Wrote = Wrote {
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
pub(crate) const A_DEPENDS_ON_PATCHED_B: Wrote = Wrote {
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
pub(crate) const TWO_MEMBERS: Wrote = Wrote {
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
pub(crate) const A_ALONE: Wrote = Wrote {
    relation: languages::Relation::WorkspaceMember,
    reading: Reading {
        rows: &[&[want_file("a/Cargo.toml"), want_file("a/Cargo.toml")]],
        causes: &[],
    },
};

/// Both manifest relations, read in part because the one manifest was refused.
pub(crate) const REFUSED: &[Wrote] = &[
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

/// The request of a run over the disk that lists `files` and asks for `relation`.
pub(crate) fn request_of(
    manifests: &[languages::ManifestFile],
    notes: &[languages::Bytes],
    files: &[languages::FilePath],
    relation: languages::Relation,
) -> languages::ProjectRequest {
    languages::ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        repository: languages::Repository::Disk,
        root: RootPath(".".into()),
        manifests: manifests.to_vec(),
        notes: notes.to_vec(),
        files: files.to_vec(),
        wanted: vec![relation],
    }
}

/// The contract over a repository holding these files, read by the real manifest and matching adapters.
pub(crate) fn use_case(held: Vec<(&'static str, &'static str)>) -> ProjectContract {
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
