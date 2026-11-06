//! The files the `project` contract reads beyond the manifests it is handed, by where they lie around a repository.

use super::project::{manifest, workspace};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::SHOWN;
use crate::inbound::contracts::project::contract::ProjectContract;
use crate::outbound::conditions::factory::cargo_platform;
use crate::outbound::job_scheduling::fixtures::fake::JobSchedulingFake;
use crate::outbound::manifest::factory::toml;
use crate::outbound::pattern_matching::factory::globset;
use crate::outbound::repository::interface::RepositoryPort;
use ryo::languages::contract::{Reading, written};
use ryo::languages::{self, RootPath};
use std::sync::Arc;

/// What the contract fails with.
type Failure = languages::ProjectError;

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
            search_scope: Default::default(),
            given_inventory: Default::default(),
            observed_entries: Default::default(),
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
pub(crate) fn workspace_with(member: &str) -> tempfile::TempDir {
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
pub(crate) fn members_at(
    contract: &ProjectContract,
    root: &std::path::Path,
    member: &str,
) -> Reading {
    let (mut out, mut symbols) = (
        languages::contract::Collect::default(),
        languages::Symbols::default(),
    );
    let resolved = languages::contract::read_project(
        contract,
        &languages::ProjectRequest {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            observed_entries: Default::default(),
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

#[test]
fn succeeds() {
    run(CONSULTED, Word::Succeeds, consulted);
}
