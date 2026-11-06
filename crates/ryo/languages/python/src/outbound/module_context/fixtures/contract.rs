//! What every `module_context` adapter must pass.

use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use crate::outbound::module_context::interface::ModuleContextPort;
use ryo::languages::{FilePath, ProjectError, ProjectRequest, Relation, Repository, RootPath};

/// A root no disk lookup can use: a Given request must never touch it.
const UNUSABLE_ROOT: &str = "invalid\0disk-root";
/// A source file of a package with a marker beside it.
const PACKAGE_MARKER: &str = "pkg/__init__.py";
/// A module of that package.
const PACKAGE_MODULE: &str = "pkg/mod.py";
/// The dotted name the module is known by.
const PACKAGE_MODULE_NAME: &str = "pkg.mod";

/// What a row of the contract asks of an adapter.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Ask {
    /// A Given lookup of a file with no package marker.
    Standalone,
    /// A Given lookup of a marker and a module whose root name is unknown.
    UnnamedRoot,
    /// A Given lookup of a module in a package with a known name.
    NamedPackage,
    /// A Disk lookup whose marker cannot be inspected.
    UnusableDiskRoot,
}

/// How a lookup settled, without any filesystem observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Settled {
    /// No row, nothing missing, nothing consulted.
    Standalone,
    /// No row because a root name is unknown, nothing consulted.
    UnknownRoot,
    /// Rows known, nothing missing, nothing consulted.
    Named,
    /// Any other outcome.
    Unexpected,
}

/// The contract's rows: a name, one case word, a question and the expected answer or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Ask, Settled, ProjectError>] = &[
    Case {
        name: CaseName("a_given_file_with_no_marker_is_standalone_and_nothing_is_consulted"),
        word: Word::Succeeds,
        input: || Ask::Standalone,
        expect: Expect::Value(Settled::Standalone),
    },
    Case {
        name: CaseName("a_given_marker_without_a_root_name_is_partial"),
        word: Word::Succeeds,
        input: || Ask::UnnamedRoot,
        expect: Expect::Value(Settled::UnknownRoot),
    },
    Case {
        name: CaseName("a_given_package_names_its_modules"),
        word: Word::Succeeds,
        input: || Ask::NamedPackage,
        expect: Expect::Value(Settled::Named),
    },
    Case {
        name: CaseName("a_disk_lookup_that_cannot_inspect_a_marker_is_a_context_error"),
        word: Word::Refuses,
        input: || Ask::UnusableDiskRoot,
        expect: Expect::Refusal(|error| selects(matches!(error, ProjectError::Context { .. }))),
    },
];

/// The request that looks the files up in a repository of the given kind.
pub(crate) fn request(repository: Repository, files: &[&str]) -> ProjectRequest {
    ProjectRequest {
        search_scope: Default::default(),
        given_inventory: Default::default(),
        observed_entries: Default::default(),
        root: RootPath(UNUSABLE_ROOT.into()),
        repository,
        manifests: vec![],
        files: files.iter().map(|path| FilePath((*path).into())).collect(),
        notes: vec![],
        wanted: vec![Relation::ModuleIdentity],
    }
}

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl ModuleContextPort) {
    let subject = |ask: Ask| -> Result<Settled, ProjectError> {
        let (repository, files): (Repository, &[&str]) = match ask {
            Ask::Standalone => (Repository::Given, &["mod.py"]),
            Ask::UnnamedRoot => (Repository::Given, &["__init__.py", "mod.py"]),
            Ask::NamedPackage => (Repository::Given, &[PACKAGE_MARKER, PACKAGE_MODULE]),
            Ask::UnusableDiskRoot => (Repository::Disk, &["mod.py"]),
        };
        let contexts = adapter.resolve(&request(repository, files))?;
        let silent = contexts.consulted.is_empty();
        Ok(match (contexts.rows.is_empty(), contexts.partial.0, silent) {
            (true, false, true) => Settled::Standalone,
            (true, true, true) => Settled::UnknownRoot,
            (false, false, true) => {
                assert_eq!(contexts.rows[1].path.0, PACKAGE_MODULE_NAME);
                Settled::Named
            }
            _ => Settled::Unexpected,
        })
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
