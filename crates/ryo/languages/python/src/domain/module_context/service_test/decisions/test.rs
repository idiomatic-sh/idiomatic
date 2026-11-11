//! Pure module decisions preserve marker boundaries and Given isolation.

use super::fixtures::{
    disk_requests_every_marker_then_retains_initializer_component,
    dotted_nonpackage_directory_does_not_create_module_context,
    given_knows_nested_packages_but_never_an_unnamed_root_package,
    paths_above_root_remain_real_relative_dependencies,
    raw_basenames_and_directory_components_are_not_filename_stems,
    root_basename_preserves_contiguous_boundary_and_namespace_prefix,
    standalone_is_complete_empty_and_pyi_is_not_a_marker,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which scenario a row runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// The `disk_requests_every_marker_then_retains_initializer_component` scenario.
    DiskRequestsEveryMarkerThenRetainsInitializerComponent,
    /// The `standalone_is_complete_empty_and_pyi_is_not_a_marker` scenario.
    StandaloneIsCompleteEmptyAndPyiIsNotAMarker,
    /// The `given_knows_nested_packages_but_never_an_unnamed_root_package` scenario.
    GivenKnowsNestedPackagesButNeverAnUnnamedRootPackage,
    /// The `paths_above_root_remain_real_relative_dependencies` scenario.
    PathsAboveRootRemainRealRelativeDependencies,
    /// The `raw_basenames_and_directory_components_are_not_filename_stems` scenario.
    RawBasenamesAndDirectoryComponentsAreNotFilenameStems,
    /// The `dotted_nonpackage_directory_does_not_create_module_context` scenario.
    DottedNonpackageDirectoryDoesNotCreateModuleContext,
    /// The `root_basename_preserves_contiguous_boundary_and_namespace_prefix` scenario.
    RootBasenamePreservesContiguousBoundaryAndNamespacePrefix,
}
/// What a row is given.
type Input = Given;
/// What the scenario observed, each check in order, joined by `|`.
type Value = &'static str;
/// A scenario is never refused.
type Failure = ();

/// The rows of this group's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("disk_requests_every_marker_then_retains_initializer_component"),
        word: Word::Succeeds,
        input: || Given::DiskRequestsEveryMarkerThenRetainsInitializerComponent,
        expect: Expect::Value(
            "Need(\"/boundary/pkg/sub/__init__.py\")|Need(\"/boundary/__init__.py\")|Known(Some(PackageModule { path: ModuleDottedPath(\"pkg.sub.__init__\"), package_root_name: PackageRootName(\"pkg\") }))",
        ),
    },
    Case {
        name: CaseName("standalone_is_complete_empty_and_pyi_is_not_a_marker"),
        word: Word::Succeeds,
        input: || Given::StandaloneIsCompleteEmptyAndPyiIsNotAMarker,
        expect: Expect::Value("Known(None)|Known(None)"),
    },
    Case {
        name: CaseName("given_knows_nested_packages_but_never_an_unnamed_root_package"),
        word: Word::Succeeds,
        input: || Given::GivenKnowsNestedPackagesButNeverAnUnnamedRootPackage,
        expect: Expect::Value(
            "Known(Some(PackageModule { path: ModuleDottedPath(\"pkg.mod\"), package_root_name: PackageRootName(\"pkg\") }))|Unknown",
        ),
    },
    Case {
        name: CaseName("paths_above_root_remain_real_relative_dependencies"),
        word: Word::Succeeds,
        input: || Given::PathsAboveRootRemainRealRelativeDependencies,
        expect: Expect::Value("Some(\"../__init__.py\")"),
    },
    Case {
        name: CaseName("raw_basenames_and_directory_components_are_not_filename_stems"),
        word: Word::Succeeds,
        input: || Given::RawBasenamesAndDirectoryComponentsAreNotFilenameStems,
        expect: Expect::Value(
            "Some(Basenames { module: NameText(\"mod.with.dot\"), package: NameText(\"nested.name\") })|Known(Some(PackageModule { path: ModuleDottedPath(\"pkg.with.dot.nested.name.mod.with.dot\"), package_root_name: PackageRootName(\"pkg.with.dot\") }))|Some(Basenames { module: NameText(\"__init__\"), package: NameText(\"pkg.with.dot\") })|None",
        ),
    },
    Case {
        name: CaseName("dotted_nonpackage_directory_does_not_create_module_context"),
        word: Word::Succeeds,
        input: || Given::DottedNonpackageDirectoryDoesNotCreateModuleContext,
        expect: Expect::Value("Known(None)|Known(None)"),
    },
    Case {
        name: CaseName("root_basename_preserves_contiguous_boundary_and_namespace_prefix"),
        word: Word::Succeeds,
        input: || Given::RootBasenamePreservesContiguousBoundaryAndNamespacePrefix,
        expect: Expect::Value(
            "\"pkg.root\"|true|true|\"pkg.root\"|true|true|\"pkg.root\"|true|true|\"pkg.root\"|true|true",
        ),
    },
];

/// Runs the scenario the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    let observed = match given {
        Given::DiskRequestsEveryMarkerThenRetainsInitializerComponent => {
            disk_requests_every_marker_then_retains_initializer_component()
        }
        Given::StandaloneIsCompleteEmptyAndPyiIsNotAMarker => {
            standalone_is_complete_empty_and_pyi_is_not_a_marker()
        }
        Given::GivenKnowsNestedPackagesButNeverAnUnnamedRootPackage => {
            given_knows_nested_packages_but_never_an_unnamed_root_package()
        }
        Given::PathsAboveRootRemainRealRelativeDependencies => {
            paths_above_root_remain_real_relative_dependencies()
        }
        Given::RawBasenamesAndDirectoryComponentsAreNotFilenameStems => {
            raw_basenames_and_directory_components_are_not_filename_stems()
        }
        Given::DottedNonpackageDirectoryDoesNotCreateModuleContext => {
            dotted_nonpackage_directory_does_not_create_module_context()
        }
        Given::RootBasenamePreservesContiguousBoundaryAndNamespacePrefix => {
            root_basename_preserves_contiguous_boundary_and_namespace_prefix()
        }
    };
    Ok(Box::leak(observed.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
