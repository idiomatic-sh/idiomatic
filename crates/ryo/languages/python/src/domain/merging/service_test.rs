//! The `merging` service's cases: the manifests of one directory are one package.

use super::service::merged;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packages::types::{
    Declared, ManifestPath, Naming, Package, PackageName, Reference, Requirement, Written,
};
use ryo::languages::{DependencyKind, Optionality};

/// The manifests of a repository: each with its path, the name it declares if it declares one, and its requirements.
type Input = &'static [(&'static str, &'static str, &'static [&'static str])];
/// The packages: each with its manifest, its name and its requirements, as one line.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("manifests_of_one_directory_are_one_package_named_by_the_one_that_names_it"),
        word: Word::Succeeds,
        input: || {
            &[
                ("requirements.txt", "", &["b", "a"]),
                ("pyproject.toml", "app", &["a"]),
            ]
        },
        expect: Expect::Value("pyproject.toml app a,b"),
    },
    Case {
        name: CaseName("manifests_of_two_directories_are_two_packages"),
        word: Word::Succeeds,
        input: || {
            &[
                ("a/pyproject.toml", "a", &[]),
                ("b/pyproject.toml", "b", &[]),
            ]
        },
        expect: Expect::Value("a/pyproject.toml a ; b/pyproject.toml b"),
    },
    Case {
        name: CaseName("a_directory_that_names_nothing_and_needs_nothing_is_no_package"),
        word: Word::Succeeds,
        input: || &[("pyproject.toml", "", &[])],
        expect: Expect::Value(""),
    },
];

fn subject(manifests: Input) -> Value {
    let found = manifests
        .iter()
        .map(|(path, name, requirements)| Package {
            manifest: ManifestPath((*path).into()),
            declared: Declared {
                name: if name.is_empty() {
                    Naming::Unnamed
                } else {
                    Naming::Named(PackageName((*name).into()))
                },
                members: Vec::new(),
                excludes: Vec::new(),
                requirements: requirements
                    .iter()
                    .map(|each| Requirement {
                        written: Written((*each).into()),
                        kind: DependencyKind::Normal,
                        optional: Optionality::Required,
                        reference: Reference::Named,
                        origin: ryo::languages::DependencyOrigin::Registry,
                        specifier: crate::domain::packages::types::Specifier(Box::default()),
                        exact: ryo::languages::Flag::Lowered,
                        checkout: crate::domain::packages::types::Checkout::default(),
                        interpreter: crate::domain::packages::types::Interpreter::default(),
                    })
                    .collect(),
                sources: Vec::new(),
                overrides: Vec::new(),
            },
        })
        .collect();
    let lines: Vec<String> = merged(found)
        .iter()
        .map(|package| {
            let name = match &package.declared.name {
                Naming::Named(name) => name.0.to_string(),
                Naming::Unnamed => String::new(),
            };
            let needs: Vec<&str> = package
                .declared
                .requirements
                .iter()
                .map(|each| &*each.written.0)
                .collect();
            let line = format!("{} {name} {}", package.manifest.0, needs.join(","));
            line.trim_end().to_owned()
        })
        .collect();
    Box::leak(lines.join(" ; ").into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
