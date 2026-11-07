//! The `dependencies` service's cases, on values alone.

use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};

use super::service::{catalog_text, declared_of, replacement_text, revision_text};
use super::types::Shared;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::BindingName;
use crate::domain::targets::types::Checkout;
use crate::domain::targets::types::DependencySection;
use crate::domain::targets::types::Inheritance;
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::Patch;
use crate::domain::targets::types::Platform;
use crate::domain::targets::types::Replacement;
use crate::domain::targets::types::Revision;
use crate::domain::targets::types::Source;
use crate::domain::targets::types::Version;
use crate::domain::targets::types::WorkspaceDependency;
use crate::domain::targets::types::WrittenPath;
use crate::domain::workspaces::fixtures::answering;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// A dependency by path, renamed, optional, for a platform.
    ByPath,
    /// A dependency that takes its path from the workspace, under another name.
    Inherited,
    /// A dependency by version alone.
    ByVersion,
    /// A dependency by version that the root manifest patches to a directory, for its source.
    Patched,
    /// A dependency by version that the root's `[patch]` takes from a git repository.
    PatchedFromGit,
    /// A version requirement as a manifest writes it, empty for none: whether it admits one version only.
    Requirement(&'static str),
}

/// What a row of the cases does.
type Input = Scenario;
/// The dependencies, shown.
type Value = &'static str;
type Failure = ();

fn dependency(key: &str) -> DependencySection {
    DependencySection {
        key: BindingName(key.into()),
        kind: DependencyKind::Normal,
        platform: Section::Absent,
        version: Section::Absent,
        path: Section::Absent,
        package: Section::Absent,
        source: Source("crates-io".into()),
        origin: DependencyOrigin::Registry,
        optional: Optionality::Required,
        workspace: Inheritance::Own,
        checkout: Checkout::default(),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the case table's one subject: an arm per scenario, each reading its own fixture"
)]
fn subject(scenario: Input) -> Value {
    if let Scenario::Requirement(written) = scenario {
        let requirement = if written.is_empty() {
            Section::Absent
        } else {
            Section::Present(Version(written.into()))
        };
        return if super::service::exactness(&requirement) == ryo::languages::Flag::Raised {
            "exact"
        } else {
            "a range"
        };
    }
    let directory = RepoPath("app".into());
    let root = RepoPath(Box::default());
    let workspace = [WorkspaceDependency {
        key: BindingName("shared".into()),
        version: Section::Absent,
        path: Section::Present(WrittenPath("shared".into())),
        package: Section::Present(PackageName("shared-real".into())),
        source: Source("crates-io".into()),
        origin: DependencyOrigin::Path,
        checkout: Checkout {
            tag: Section::Present(Revision("v1.0.0".into())),
            ..Checkout::default()
        },
    }];
    // `serde` is patched for its git repository alone, `patched` for crates.io.
    let patches = [
        Patch {
            source: Source("https://github.com/serde-rs/serde".into()),
            package: PackageName("serde".into()),
            replacement: Replacement::Directory(WrittenPath("vendor/serde".into())),
        },
        Patch {
            source: Source("crates-io".into()),
            package: PackageName("patched".into()),
            replacement: Replacement::Directory(WrittenPath("vendor/patched".into())),
        },
        Patch {
            source: Source("crates-io".into()),
            package: PackageName("fromgit".into()),
            replacement: Replacement::Repository(Source("https://example.com/fromgit".into())),
        },
    ];
    let manifest = Manifest {
        dependencies: vec![match scenario {
            Scenario::ByPath => DependencySection {
                kind: DependencyKind::Dev,
                platform: Section::Present(Platform("cfg( unix )".into())),
                path: Section::Present(WrittenPath("../helper".into())),
                origin: DependencyOrigin::Path,
                optional: Optionality::Optional,
                ..dependency("helper")
            },
            Scenario::Inherited => DependencySection {
                kind: DependencyKind::Build,
                workspace: Inheritance::Workspace,
                ..dependency("shared")
            },
            Scenario::ByVersion => DependencySection {
                version: Section::Present(Version("1".into())),
                ..dependency("serde")
            },
            Scenario::PatchedFromGit => DependencySection {
                version: Section::Present(Version("1".into())),
                ..dependency("fromgit")
            },
            Scenario::Patched | Scenario::Requirement(_) => DependencySection {
                version: Section::Present(Version("1".into())),
                ..dependency("patched")
            },
        }],
        ..Manifest::default()
    };
    let found = answering(&[], |looking| {
        declared_of(
            &directory,
            &manifest,
            Shared {
                directory: &root,
                dependencies: &workspace,
                patches: &patches,
            },
            &mut looking.platforms,
        )
    });
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(
        found
            .into_iter()
            .map(|each| {
                format!(
                    "{} {:?} as {} of {} {:?} [{}] {} [{}] <{}> @{},{},{}",
                    each.target.as_ref().map_or("", |target| &*target.0),
                    each.origin,
                    each.binding,
                    each.package,
                    each.kind,
                    each.condition,
                    (each.optional == Optionality::Optional),
                    catalog_text(each.inheritance).0,
                    replacement_text(&each.replacement).0,
                    revision_text(&each.checkout.branch).0,
                    revision_text(&each.checkout.tag).0,
                    revision_text(&each.checkout.rev).0,
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
            .into_boxed_str(),
    )
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_dependency_by_path_names_the_manifest_its_kind_its_platform_and_its_optionality",
        ),
        word: Word::Succeeds,
        input: || Scenario::ByPath,
        expect: Expect::Value(
            "helper/Cargo.toml Path as helper of helper Dev [cfg(unix)] true [] <> @,,",
        ),
    },
    Case {
        name: CaseName(
            "a_dependency_from_the_workspace_takes_its_path_package_and_revision_from_it_and_says_so",
        ),
        word: Word::Succeeds,
        input: || Scenario::Inherited,
        expect: Expect::Value(
            "shared/Cargo.toml Path as shared of shared-real Build [] false [workspace = true] <> @,v1.0.0,",
        ),
    },
    Case {
        name: CaseName("a_dependency_by_version_alone_is_from_a_registry_and_names_no_directory"),
        word: Word::Succeeds,
        input: || Scenario::ByVersion,
        expect: Expect::Value(" Registry as serde of serde Normal [] false [] <> @,,"),
    },
    Case {
        name: CaseName(
            "a_dependency_by_version_the_root_patches_for_its_source_names_the_patch_directory_as_its_target_and_replacement",
        ),
        word: Word::Succeeds,
        input: || Scenario::Patched,
        expect: Expect::Value(
            "vendor/patched/Cargo.toml Registry as patched of patched Normal [] false [] <vendor/patched> @,,",
        ),
    },
    Case {
        name: CaseName(
            "a_dependency_the_root_patches_from_a_git_repository_names_its_url_as_replacement_and_no_directory",
        ),
        word: Word::Succeeds,
        input: || Scenario::PatchedFromGit,
        expect: Expect::Value(
            " Registry as fromgit of fromgit Normal [] false [] <https://example.com/fromgit> @,,",
        ),
    },
    Case {
        name: CaseName("an_equals_with_a_whole_version_is_exact"),
        word: Word::Succeeds,
        input: || Scenario::Requirement("=1.2.3"),
        expect: Expect::Value("exact"),
    },
    Case {
        name: CaseName("an_equals_with_a_pre_release_among_other_comparators_is_exact"),
        word: Word::Succeeds,
        input: || Scenario::Requirement(">=1, = 1.2.3-rc.1"),
        expect: Expect::Value("exact"),
    },
    Case {
        name: CaseName("a_bare_version_is_a_caret_range"),
        word: Word::Succeeds,
        input: || Scenario::Requirement("1.2.3"),
        expect: Expect::Value("a range"),
    },
    Case {
        name: CaseName("an_equals_with_a_partial_version_admits_its_patches"),
        word: Word::Succeeds,
        input: || Scenario::Requirement("=1.2"),
        expect: Expect::Value("a range"),
    },
    Case {
        name: CaseName("a_minimum_is_a_range"),
        word: Word::Succeeds,
        input: || Scenario::Requirement(">=1.2.3"),
        expect: Expect::Value("a range"),
    },
    Case {
        name: CaseName("no_requirement_is_not_exact"),
        word: Word::Succeeds,
        input: || Scenario::Requirement(""),
        expect: Expect::Value("a range"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
