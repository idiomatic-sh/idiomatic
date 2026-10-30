//! Workspace members and catalogs, through the project contract: pnpm members, default and named catalogs, Bun catalogs, a missing entry
//! and npm and Yarn workspaces unchanged, each with the control that shows what the case rests on.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifests::collections::manifests::file_package::item::FilePackageItem;
use crate::domain::manifests::collections::manifests::package_dependency::item::PackageDependencyItem;
use crate::domain::manifests::collections::manifests::workspace_member::item::WorkspaceMemberItem;
use crate::fixtures::projects::{read, shown};
use crate::fixtures::workspaces::{
    BUN_NO_CATALOG, BUN_TOP_LEVEL_CATALOGS, BUN_WORKSPACES_CATALOG, MEMBER_OVERRIDES,
    MISSING_ENTRY, MONOREPO, NPM_OVERRIDES, NPM_WORKSPACES, PNPM_DEFAULT_CATALOG, PNPM_MEMBERS,
    PNPM_NAMED_CATALOG, PNPM_NAMED_ONLY, PNPM_NO_WORKSPACE_FILE, PNPM_OVERRIDES, PNPM_PATH_CATALOG,
    PNPM_PLAIN_CATALOG, YARN_WORKSPACES,
};

type Files = &'static [(&'static str, &'static str)];

/// What a row asks of the project contract.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The workspace-member rows, as (workspace, package) file places.
    Members(Files),
    /// The package-dependency rows, as (source, target, binding, package) and how many facts were reported partial.
    Dependencies(Files),
    /// The file-package rows, as (file, package) file places.
    Ownership(Files),
    /// Whether a dependency spec admits one version only: past a `workspace:` or an `npm:` alias, a whole version.
    Exactness(&'static str),
    /// The package-dependency rows, as each binding with its catalog's spelling, its revision and its replacement where it has one.
    Written(Files),
}

type Input = Ask;
/// The rows in one text, then the number of partial reports.
type Value = (&'static str, usize);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("pnpm_workspace_file_declares_members_and_negation_excludes"),
        word: Word::Succeeds,
        input: || Ask::Members(PNPM_MEMBERS),
        expect: Expect::Value(("0>1", 0)),
    },
    Case {
        name: CaseName("without_the_pnpm_workspace_file_no_member_is_declared"),
        word: Word::Succeeds,
        input: || Ask::Members(PNPM_NO_WORKSPACE_FILE),
        expect: Expect::Value(("", 0)),
    },
    Case {
        name: CaseName("pnpm_default_catalog_gives_the_dependency_its_package"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(PNPM_DEFAULT_CATALOG),
        expect: Expect::Value(("1>2 core real", 0)),
    },
    Case {
        name: CaseName("a_path_in_a_catalog_is_from_the_workspace_file_not_the_package_using_it"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(PNPM_PATH_CATALOG),
        expect: Expect::Value(("1>2 lib lib", 0)),
    },
    Case {
        name: CaseName("pnpm_default_catalog_plain_range_keeps_the_dependency_name"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(PNPM_PLAIN_CATALOG),
        expect: Expect::Value(("1>2 core core", 0)),
    },
    Case {
        name: CaseName("a_resolved_catalog_spec_keeps_the_spelling_the_manifest_writes"),
        word: Word::Succeeds,
        input: || Ask::Written(PNPM_NAMED_CATALOG),
        expect: Expect::Value(("core catalog=catalog:legacy", 0)),
    },
    Case {
        name: CaseName(
            "a_root_override_of_a_package_alone_replaces_the_dependency_and_a_git_spec_names_its_revision",
        ),
        word: Word::Succeeds,
        input: || Ask::Written(NPM_OVERRIDES),
        expect: Expect::Value((
            "@scope/lib replacement=1.0.0, core replacement=2.0.0, left, plain revision=main, ranged, right replacement=4.0.0",
            0,
        )),
    },
    Case {
        name: CaseName("a_members_own_override_is_not_applied_only_the_roots_is"),
        word: Word::Succeeds,
        input: || Ask::Written(MEMBER_OVERRIDES),
        expect: Expect::Value(("core replacement=2.0.0, own", 0)),
    },
    Case {
        name: CaseName("a_pnpm_override_under_a_parent_replaces_no_dependency_of_its_own"),
        word: Word::Succeeds,
        input: || Ask::Written(PNPM_OVERRIDES),
        expect: Expect::Value(("core replacement=2.0.0, deep", 0)),
    },
    Case {
        name: CaseName("pnpm_named_catalog_gives_the_dependency_its_package"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(PNPM_NAMED_CATALOG),
        expect: Expect::Value(("1>2 core real", 0)),
    },
    Case {
        name: CaseName("a_named_catalog_does_not_answer_the_default_catalog"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(PNPM_NAMED_ONLY),
        expect: Expect::Value(("", 1)),
    },
    Case {
        name: CaseName("a_catalog_reference_to_a_missing_entry_is_unknown_and_reported"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(MISSING_ENTRY),
        expect: Expect::Value(("", 1)),
    },
    Case {
        name: CaseName("bun_catalog_beside_the_workspace_packages_resolves"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(BUN_WORKSPACES_CATALOG),
        expect: Expect::Value(("1>2 core real", 0)),
    },
    Case {
        name: CaseName("bun_top_level_named_catalog_resolves"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(BUN_TOP_LEVEL_CATALOGS),
        expect: Expect::Value(("1>2 core real", 0)),
    },
    Case {
        name: CaseName("without_a_bun_catalog_the_reference_is_unknown_and_reported"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(BUN_NO_CATALOG),
        expect: Expect::Value(("", 1)),
    },
    Case {
        name: CaseName("npm_workspaces_list_declares_members"),
        word: Word::Succeeds,
        input: || Ask::Members(NPM_WORKSPACES),
        expect: Expect::Value(("0>1 0>2", 0)),
    },
    Case {
        name: CaseName("npm_workspaces_keep_their_dependency_edges"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(NPM_WORKSPACES),
        expect: Expect::Value(("1>2 core core", 0)),
    },
    Case {
        name: CaseName("yarn_workspaces_object_declares_members"),
        word: Word::Succeeds,
        input: || Ask::Members(YARN_WORKSPACES),
        expect: Expect::Value(("0>1 0>2", 0)),
    },
    Case {
        name: CaseName("yarn_workspaces_keep_their_dependency_edges"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(YARN_WORKSPACES),
        expect: Expect::Value(("1>2 core core", 0)),
    },
    Case {
        name: CaseName("monorepo_workspace_members_are_the_packages_under_the_root"),
        word: Word::Succeeds,
        input: || Ask::Members(MONOREPO),
        expect: Expect::Value(("0>1 0>3", 0)),
    },
    Case {
        name: CaseName("monorepo_dependency_is_the_app_on_the_ui_package"),
        word: Word::Succeeds,
        input: || Ask::Dependencies(MONOREPO),
        expect: Expect::Value(("1>3 ui ui", 0)),
    },
    Case {
        name: CaseName("monorepo_files_belong_to_their_nearest_package"),
        word: Word::Succeeds,
        input: || Ask::Ownership(MONOREPO),
        expect: Expect::Value(("0>0 1>1 2>1 3>3 4>3 5>0", 0)),
    },
    Case {
        name: CaseName("a_whole_version_is_exact"),
        word: Word::Succeeds,
        input: || Ask::Exactness("1.2.3"),
        expect: Expect::Value(("exact", 0)),
    },
    Case {
        name: CaseName("an_equals_or_a_v_before_a_whole_version_is_exact"),
        word: Word::Succeeds,
        input: || Ask::Exactness("=v1.2.3-beta.1"),
        expect: Expect::Value(("exact", 0)),
    },
    Case {
        name: CaseName("an_alias_to_a_whole_version_is_exact"),
        word: Word::Succeeds,
        input: || Ask::Exactness("npm:@scope/other@2.0.0"),
        expect: Expect::Value(("exact", 0)),
    },
    Case {
        name: CaseName("a_workspace_member_at_a_whole_version_is_exact"),
        word: Word::Succeeds,
        input: || Ask::Exactness("workspace:1.0.0"),
        expect: Expect::Value(("exact", 0)),
    },
    Case {
        name: CaseName("a_caret_is_a_range"),
        word: Word::Succeeds,
        input: || Ask::Exactness("^4.6.0"),
        expect: Expect::Value(("a range", 0)),
    },
    Case {
        name: CaseName("a_hyphen_range_of_whole_versions_is_a_range"),
        word: Word::Succeeds,
        input: || Ask::Exactness("1.2.3 - 2.0.0"),
        expect: Expect::Value(("a range", 0)),
    },
    Case {
        name: CaseName("a_workspace_star_a_tag_and_a_partial_version_are_not_exact"),
        word: Word::Succeeds,
        input: || Ask::Exactness("workspace:*"),
        expect: Expect::Value(("a range", 0)),
    },
    Case {
        name: CaseName("a_directory_is_no_version"),
        word: Word::Succeeds,
        input: || Ask::Exactness("file:../core"),
        expect: Expect::Value(("a range", 0)),
    },
];

fn subject(ask: Input) -> Value {
    match ask {
        Ask::Exactness(spec) => {
            let exact =
                super::service::exactness(&crate::domain::packages::types::Version(spec.into()));
            (
                if exact == ryo::languages::Flag::Raised {
                    "exact"
                } else {
                    "a range"
                },
                0,
            )
        }
        Ask::Members(files) => {
            let (written, _) = read::<WorkspaceMemberItem>(files);
            (
                Box::leak(
                    written
                        .rows
                        .iter()
                        .map(|row| format!("{}>{}", row.workspace.0, row.package.0))
                        .collect::<Vec<_>>()
                        .join(" ")
                        .into_boxed_str(),
                ),
                written.partial,
            )
        }
        Ask::Ownership(files) => {
            let (written, _) = read::<FilePackageItem>(files);
            (
                Box::leak(
                    written
                        .rows
                        .iter()
                        .map(|row| format!("{}>{}", row.path.0, row.package.0))
                        .collect::<Vec<_>>()
                        .join(" ")
                        .into_boxed_str(),
                ),
                written.partial,
            )
        }
        Ask::Written(files) => {
            let (written, symbols) = read::<PackageDependencyItem>(files);
            (
                Box::leak(
                    written
                        .rows
                        .iter()
                        .map(|row| {
                            let mut line = shown(&symbols, row.binding);
                            for (column, value) in [
                                ("catalog", row.catalog),
                                ("revision", row.revision),
                                ("replacement", row.replacement),
                            ] {
                                let value = shown(&symbols, value);
                                if !value.is_empty() {
                                    line.push(' ');
                                    line.push_str(column);
                                    line.push('=');
                                    line.push_str(&value);
                                }
                            }
                            line
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                        .into_boxed_str(),
                ),
                written.partial,
            )
        }
        Ask::Dependencies(files) => {
            let (written, symbols) = read::<PackageDependencyItem>(files);
            (
                Box::leak(
                    written
                        .rows
                        .iter()
                        .map(|row| {
                            format!(
                                "{}>{} {} {}",
                                row.source.0,
                                row.target.0,
                                shown(&symbols, row.binding),
                                shown(&symbols, row.package)
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                        .into_boxed_str(),
                ),
                written.partial,
            )
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
