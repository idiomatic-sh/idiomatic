//! The `dependencies` service's cases: a requirement is found by name, by the directory it names, or by the directory its manifest sends
//! the name to, and a requirement that names no package of the repository has no target; each says where its package comes from.

use super::service::dependencies_of;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::{
    Checkout, Declared, Interpreter, PackageLocation, ManifestPath, Marker, Naming, Package, PackageName,
    Packages, Redirect, Reference, Requirement, Revision, Written,
};
use crate::domain::patterns::types::Pattern;
use ryo::languages::{DependencyKind, DependencyOrigin, Optionality};

/// One source the manifest gives the package: the directory it sends to (none when outside the repository), its marker and the tag of
/// a git repository it names (empty for none).
type Source = (Option<&'static str>, &'static str, &'static str);
/// Whether the package with the requirement is the root of its install, or a member of a workspace at the root.
#[derive(Clone, Copy, Debug)]
enum Holding {
    /// No workspace holds it.
    Alone,
    /// A workspace at the root holds it, and writes no overrides of its own.
    Member,
}

/// What a case gives: the requirement's text and reference, the redirects its manifest holds, the requirements its manifest lists
/// in `override-dependencies`, and whether a workspace holds it.
type Input = (
    &'static str,
    Option<&'static str>,
    &'static [Source],
    &'static [&'static str],
    Holding,
);
/// The target manifests it is found at (empty for none), each with where it comes from and its condition, joined.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_requirement_is_found_by_its_normalized_name"),
        word: Word::Succeeds,
        input: || ("Lib_One>=1", None, &[], &[], Holding::Alone),
        expect: Expect::Value("libs/one/pyproject.toml:Registry"),
    },
    Case {
        name: CaseName("a_requirement_on_a_package_outside_the_repository_has_no_target"),
        word: Word::Succeeds,
        input: || ("requests", None, &[], &[], Holding::Alone),
        expect: Expect::Value(":Registry"),
    },
    Case {
        name: CaseName("a_requirement_with_a_directory_is_found_there_whatever_it_is_called"),
        word: Word::Succeeds,
        input: || ("../x", Some("libs/two"), &[], &[], Holding::Alone),
        expect: Expect::Value("libs/two/pyproject.toml:Path"),
    },
    Case {
        name: CaseName("a_redirect_sends_a_name_to_a_directory"),
        word: Word::Succeeds,
        input: || {
            (
                "lib-one",
                None,
                &[(Some("libs/two"), "", "")],
                &[],
                Holding::Alone,
            )
        },
        expect: Expect::Value("libs/two/pyproject.toml:Path"),
    },
    Case {
        name: CaseName(
            "a_name_sent_outside_the_repository_has_no_target_whatever_a_package_of_it_is_called",
        ),
        word: Word::Succeeds,
        input: || ("lib-one", None, &[(None, "", "")], &[], Holding::Alone),
        expect: Expect::Value(":Path"),
    },
    Case {
        name: CaseName("each_conditional_source_is_an_edge_with_its_condition"),
        word: Word::Succeeds,
        input: || {
            (
                "lib-one",
                None,
                &[
                    (Some("libs/one"), "sys_platform == 'win32'", ""),
                    (Some("libs/two"), "sys_platform == 'linux'", ""),
                ],
                &[],
                Holding::Alone,
            )
        },
        expect: Expect::Value(
            "libs/one/pyproject.toml:Path ; sys_platform == 'win32',libs/two/pyproject.toml:Path ; sys_platform == 'linux'",
        ),
    },
    Case {
        name: CaseName("a_source_that_names_a_tag_of_a_git_repository_gives_it_to_the_requirement"),
        word: Word::Succeeds,
        input: || {
            (
                "lib-one",
                None,
                &[(None, "", "v1.0.0")],
                &[],
                Holding::Alone,
            )
        },
        expect: Expect::Value(":Path tag=v1.0.0"),
    },
    Case {
        name: CaseName("an_override_of_the_same_name_is_what_replaces_the_requirement"),
        word: Word::Succeeds,
        input: || {
            (
                "Lib_One>=1",
                None,
                &[],
                &["requests==2", "lib-one==2.0"],
                Holding::Alone,
            )
        },
        expect: Expect::Value("libs/one/pyproject.toml:Registry replaced by lib-one==2.0"),
    },
    Case {
        name: CaseName("a_members_own_override_is_not_applied_only_the_workspace_roots_is"),
        word: Word::Succeeds,
        input: || ("Lib_One>=1", None, &[], &["lib-one==2.0"], Holding::Member),
        expect: Expect::Value("libs/one/pyproject.toml:Registry"),
    },
    Case {
        name: CaseName("a_source_condition_joins_the_requirements_own"),
        word: Word::Succeeds,
        input: || {
            (
                "lib-one ; python_version >= '3.9'",
                None,
                &[(Some("libs/two"), "sys_platform == 'linux'", "")],
                &[],
                Holding::Alone,
            )
        },
        expect: Expect::Value(
            "libs/two/pyproject.toml:Path ; (python_version >= '3.9') and (sys_platform == 'linux')",
        ),
    },
];

/// A package named `name` in the directory of `manifest`.
fn named(manifest: &str, name: &str) -> Package {
    Package {
        manifest: ManifestPath(manifest.into()),
        declared: Declared {
            name: Naming::Named(PackageName(name.into())),
            members: Vec::new(),
            excludes: Vec::new(),
            requirements: Vec::new(),
            sources: Vec::new(),
            overrides: Vec::new(),
        },
    }
}

fn subject((text, directory, redirects, overrides, holding): Input) -> Value {
    let mut app = match holding {
        Holding::Alone => named("pyproject.toml", "app"),
        Holding::Member => named("apps/app/pyproject.toml", "app"),
    };
    app.declared.requirements.push(Requirement {
        written: Written(text.into()),
        kind: DependencyKind::Normal,
        optional: Optionality::Required,
        reference: directory.map_or(Reference::Named, |each| {
            Reference::Located(PackageLocation(each.into()))
        }),
        origin: directory.map_or(DependencyOrigin::Registry, |_| DependencyOrigin::Path),
        specifier: crate::domain::packages::service::specifier_of(&Written(text.into())),
        exact: ryo::languages::Flag::Lowered,
        checkout: Checkout::default(),
        interpreter: Interpreter::default(),
    });
    app.declared.overrides = overrides
        .iter()
        .map(|each| Written((*each).into()))
        .collect();
    for (location, marker, tag) in redirects {
        app.declared.sources.push(Redirect {
            name: PackageName("lib_one".into()),
            location: location.map(|each| PackageLocation(each.into())),
            origin: DependencyOrigin::Path,
            marker: Marker((*marker).into()),
            checkout: Checkout {
                tag: Revision((*tag).into()),
                ..Checkout::default()
            },
        });
    }
    let packages = Packages {
        packages: vec![
            app,
            named("libs/one/pyproject.toml", "lib-one"),
            named("libs/two/pyproject.toml", "lib-two"),
        ]
        .into_iter()
        .chain(match holding {
            Holding::Alone => None,
            Holding::Member => {
                let mut root = named("pyproject.toml", "root");
                root.declared.members.push(Pattern("apps/*".into()));
                Some(root)
            }
        })
        .collect(),
        refused: Vec::new(),
    };
    let targets: Vec<String> = dependencies_of(&packages)
        .into_iter()
        .map(|each| {
            let target = each.target.as_ref().map_or("", |target| &*target.0);
            // A tag and a replacement are shown only where there is one.
            let tag = if each.checkout.tag.0.is_empty() {
                String::new()
            } else {
                format!(" tag={}", each.checkout.tag.0)
            };
            let replacement = if each.replacement.0.is_empty() {
                String::new()
            } else {
                format!(" replaced by {}", each.replacement.0)
            };
            if each.condition.0.is_empty() {
                format!("{target}:{:?}{tag}{replacement}", each.origin)
            } else {
                format!(
                    "{target}:{:?} ; {}{tag}{replacement}",
                    each.origin, each.condition.0
                )
            }
        })
        .collect();
    Box::leak(targets.join(",").into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
