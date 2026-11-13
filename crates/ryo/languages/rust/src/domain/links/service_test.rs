//! The `links` service's cases, on values alone: facts of files go in, and the links between the files come out.

use std::collections::{BTreeMap, BTreeSet};

use super::service::{draw, examine, gaps_in, merged};
use super::types::{ExternCrate, Externs, Linked, Run, SourceDependencyLink};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::module_tree::types::{
    Body, Cleanliness, Facts, ModDeclaration, Segment, Spelling, Written,
};
use crate::domain::package_index::service::{index, kept_files};
use crate::domain::package_index::types::ListIndex;
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use ryo::languages::{FilePath, Offset};

/// A scenario: a repository of files with their facts, and what to link.
pub(super) type Input = fn() -> Repository;
/// The links as `source -> target`, then the files analyzed, all by path.
pub(super) type Value = &'static str;
pub(super) type Failure = ();

/// The files of a repository, the crate roots Cargo declares by package directory, the outputs, and the facts of each source file.
/// `externs` are the libraries each package depends on, as `(package directory, name its code writes, library's package directory,
/// library root)`.
pub(super) struct Repository {
    files: Vec<&'static str>,
    roots: Vec<(&'static str, &'static str)>,
    outputs: Vec<&'static str>,
    facts: Vec<(&'static str, Facts)>,
    externs: Vec<(&'static str, &'static str, &'static str, &'static str)>,
}

pub(super) fn declares(names: &[&str]) -> Vec<ModDeclaration> {
    names
        .iter()
        .map(|name| ModDeclaration {
            gate: crate::domain::module_tree::types::Gate::Always,
            reach: crate::domain::module_tree::types::Exposure::Dependents,
            around: Vec::new(),
            name: Segment((*name).into()),
            body: Body::File,
            path: Section::Absent,
        })
        .collect()
}

pub(super) fn writes(segments: &[&str]) -> Written {
    Written {
        segments: segments
            .iter()
            .map(|each| Segment((*each).into()))
            .collect(),
        enclosing: Vec::new(),
        start: Offset(1),
        end: Offset(2),
        text: Spelling(segments.join("::").into()),
        bound: None,
        scope: None,
    }
}

pub(super) fn facts(declared: Vec<ModDeclaration>, written: Vec<Written>) -> Facts {
    Facts {
        clean: Cleanliness::Clean,
        declared,
        written,
    }
}

/// A package `p` rooted at `p/src/lib.rs`.
pub(super) fn package(files: Vec<&'static str>, facts: Vec<(&'static str, Facts)>) -> Repository {
    Repository {
        files,
        roots: vec![("p", "p/src/lib.rs")],
        outputs: Vec::new(),
        facts,
        externs: Vec::new(),
    }
}

fn a_path_from_the_crate_root() -> Repository {
    package(
        vec!["p/Cargo.toml", "p/src/b.rs", "p/src/c.rs", "p/src/lib.rs"],
        vec![
            ("p/src/lib.rs", facts(declares(&["b", "c"]), Vec::new())),
            (
                "p/src/b.rs",
                facts(Vec::new(), vec![writes(&["crate", "c", "thing"])]),
            ),
            ("p/src/c.rs", facts(Vec::new(), Vec::new())),
        ],
    )
}

fn a_child_named_bare() -> Repository {
    package(
        vec!["p/Cargo.toml", "p/src/b.rs", "p/src/lib.rs"],
        vec![
            (
                "p/src/lib.rs",
                facts(declares(&["b"]), vec![writes(&["b", "f"])]),
            ),
            ("p/src/b.rs", facts(Vec::new(), Vec::new())),
        ],
    )
}

fn a_path_from_the_module_above() -> Repository {
    package(
        vec![
            "p/Cargo.toml",
            "p/src/a.rs",
            "p/src/a/b.rs",
            "p/src/lib.rs",
            "p/src/s.rs",
        ],
        vec![
            ("p/src/lib.rs", facts(declares(&["a", "s"]), Vec::new())),
            ("p/src/a.rs", facts(declares(&["b"]), Vec::new())),
            (
                "p/src/a/b.rs",
                facts(Vec::new(), vec![writes(&["super", "super", "s", "x"])]),
            ),
            ("p/src/s.rs", facts(Vec::new(), Vec::new())),
        ],
    )
}

fn a_file_no_crate_root_declares_is_not_analyzed() -> Repository {
    package(
        vec!["p/Cargo.toml", "p/src/lib.rs", "p/src/orphan.rs"],
        vec![
            ("p/src/lib.rs", facts(Vec::new(), Vec::new())),
            (
                "p/src/orphan.rs",
                facts(Vec::new(), vec![writes(&["crate", "x"])]),
            ),
        ],
    )
}

fn a_file_with_a_syntax_error_is_not_analyzed() -> Repository {
    let mut broken = facts(Vec::new(), Vec::new());
    broken.clean = Cleanliness::Broken;
    package(
        vec!["p/Cargo.toml", "p/src/b.rs", "p/src/lib.rs"],
        vec![
            ("p/src/lib.rs", facts(declares(&["b"]), Vec::new())),
            ("p/src/b.rs", broken),
        ],
    )
}

fn a_path_into_its_own_file_is_no_dependency() -> Repository {
    package(
        vec!["p/Cargo.toml", "p/src/b.rs", "p/src/lib.rs"],
        vec![
            ("p/src/lib.rs", facts(declares(&["b"]), Vec::new())),
            (
                "p/src/b.rs",
                facts(Vec::new(), vec![writes(&["crate", "b", "x"])]),
            ),
        ],
    )
}

fn files_in_an_output_directory_belong_to_no_package() -> Repository {
    let mut made = a_path_from_the_crate_root();
    made.files.push("p/target/debug/build/gen.rs");
    made.outputs.push("p/target");
    made.facts.push((
        "p/target/debug/build/gen.rs",
        facts(Vec::new(), vec![writes(&["crate", "x"])]),
    ));
    made
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_from_the_crate_root_names_the_file_of_the_module"),
        word: Word::Succeeds,
        input: || a_path_from_the_crate_root,
        expect: Expect::Value("p/src/b.rs -> p/src/c.rs | p/src/b.rs p/src/c.rs p/src/lib.rs"),
    },
    Case {
        name: CaseName("a_child_named_bare_is_the_file_of_the_child"),
        word: Word::Succeeds,
        input: || a_child_named_bare,
        expect: Expect::Value("p/src/lib.rs -> p/src/b.rs | p/src/b.rs p/src/lib.rs"),
    },
    Case {
        name: CaseName("super_goes_to_the_module_above"),
        word: Word::Succeeds,
        input: || a_path_from_the_module_above,
        expect: Expect::Value(
            "p/src/a/b.rs -> p/src/s.rs | p/src/a.rs p/src/a/b.rs p/src/lib.rs p/src/s.rs",
        ),
    },
    Case {
        name: CaseName("a_file_no_crate_root_declares_is_not_analyzed"),
        word: Word::Succeeds,
        input: || a_file_no_crate_root_declares_is_not_analyzed,
        expect: Expect::Value(" | p/src/lib.rs"),
    },
    Case {
        name: CaseName("a_file_with_a_syntax_error_is_not_analyzed"),
        word: Word::Succeeds,
        input: || a_file_with_a_syntax_error_is_not_analyzed,
        expect: Expect::Value(" | p/src/lib.rs"),
    },
    Case {
        name: CaseName("a_path_into_its_own_file_is_no_dependency"),
        word: Word::Succeeds,
        input: || a_path_into_its_own_file_is_no_dependency,
        expect: Expect::Value(" | p/src/b.rs p/src/lib.rs"),
    },
    Case {
        name: CaseName("files_in_an_output_directory_belong_to_no_package"),
        word: Word::Succeeds,
        input: || files_in_an_output_directory_belong_to_no_package,
        expect: Expect::Value("p/src/b.rs -> p/src/c.rs | p/src/b.rs p/src/c.rs p/src/lib.rs"),
    },
];

pub(super) fn subject(made: Input) -> Value {
    let made = made();
    let files: Vec<FilePath> = made
        .files
        .iter()
        .map(|each| FilePath((*each).into()))
        .collect();
    let mut roots: BTreeMap<RepoPath, BTreeSet<RepoPath>> = BTreeMap::new();
    for (package, root) in &made.roots {
        roots
            .entry(RepoPath((*package).into()))
            .or_default()
            .extend([RepoPath((*root).into())]);
    }
    let outputs: Vec<RepoPath> = made
        .outputs
        .iter()
        .map(|each| RepoPath((*each).into()))
        .collect();
    let mut known: Vec<Option<Facts>> = vec![None; files.len()];
    for (path, each) in made.facts {
        if let Ok(place) = files.binary_search_by(|file| (**file).cmp(path)) {
            known[place] = Some(each);
        }
    }
    let mut externs = Externs::default();
    for (package, name, directory, root) in made.externs {
        externs
            .by_package
            .entry(RepoPath(package.into()))
            .or_default()
            .entry(Segment(name.into()))
            .or_default()
            .push(ExternCrate {
                directory: RepoPath(directory.into()),
                root: RepoPath(root.into()),
                kind: ryo::languages::DependencyKind::Normal,
            });
    }
    let linked = link(&files, &roots, &outputs, &known, &externs);
    let links: Vec<String> = linked
        .dependencies
        .iter()
        .map(|SourceDependencyLink { source, target, .. }| {
            format!("{} -> {}", files[source.0], files[target.0])
        })
        .collect();
    let analyzed: Vec<&str> = linked
        .analyzed
        .iter()
        .map(|place| &*files[place.0])
        .collect();
    // The answer is leaked: a case compares it with text that lives for the whole run.
    let gaps = if linked.gaps.is_empty() {
        String::new()
    } else {
        format!(
            " | {}",
            linked
                .gaps
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join("; ")
        )
    };
    Box::leak(format!("{} | {}{gaps}", links.join(", "), analyzed.join(" ")).into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

/// Links the files one after the other: the same pieces the `project` contract spreads over the machine.
fn link(
    files: &[FilePath],
    roots: &BTreeMap<RepoPath, BTreeSet<RepoPath>>,
    outputs: &[RepoPath],
    facts: &[Option<Facts>],
    externs: &Externs,
) -> Linked {
    let kept = kept_files(files, outputs);
    let packages = index(&kept, roots);
    let run = Run {
        kept: &kept,
        files,
        facts,
    };
    let trees: BTreeMap<_, _> = packages
        .directories
        .iter()
        .map(|package| {
            let drawn = draw(run, package);
            (drawn.directory, drawn.trees)
        })
        .collect();
    let examined = files
        .iter()
        .enumerate()
        .map(|(place, file)| examine(run, &packages, &trees, externs, ListIndex(place), file))
        .collect();
    merged(run, gaps_in(&trees), examined)
}

#[cfg(test)]
mod aliases_test;
#[cfg(test)]
mod external_test;
