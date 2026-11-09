//! The `module_tree` service's cases, on values alone.

use std::collections::{BTreeMap, BTreeSet};

use super::service::tree;
use super::types::{
    Body, Cleanliness, EnclosingBlock, Facts, ModDeclaration, Segment, Spelling, Written,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;
use ryo::languages::Offset;

/// A scenario: a repository of files with their facts, and the crate to draw the tree of.
type Input = fn() -> Repository;
/// The modules of the tree as `module=files`, then the gaps.
type Value = &'static str;
type Failure = ();

/// The files of a repository and the facts of each source file.
struct Repository {
    files: Vec<&'static str>,
    facts: Vec<(&'static str, Facts)>,
}

fn declares(names: &[&str]) -> Vec<ModDeclaration> {
    names
        .iter()
        .map(|name| ModDeclaration {
            gate: crate::domain::module_tree::types::Gate::Always,
            around: Vec::new(),
            name: Segment((*name).into()),
            body: Body::File,
            path: Section::Absent,
        })
        .collect()
}

fn writes(segments: &[&str]) -> Written {
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

fn facts(declared: Vec<ModDeclaration>, written: Vec<Written>) -> Facts {
    Facts {
        clean: Cleanliness::Clean,
        declared,
        written,
    }
}

/// A package `p` rooted at `p/src/lib.rs`.
fn package(files: Vec<&'static str>, facts: Vec<(&'static str, Facts)>) -> Repository {
    Repository { files, facts }
}

fn a_module_file_owns_its_directory() -> Repository {
    package(
        vec![
            "p/Cargo.toml",
            "p/src/lib.rs",
            "p/src/net/mod.rs",
            "p/src/net/tcp.rs",
        ],
        vec![
            (
                "p/src/lib.rs",
                facts(declares(&["net"]), vec![writes(&["net", "tcp", "open"])]),
            ),
            ("p/src/net/mod.rs", facts(declares(&["tcp"]), Vec::new())),
            ("p/src/net/tcp.rs", facts(Vec::new(), Vec::new())),
        ],
    )
}

fn a_path_attribute_names_the_file() -> Repository {
    let mut declared = declares(&["elsewhere"]);
    declared[0].path = Section::Present(WrittenPath("../shared/x.rs".into()));
    package(
        vec!["p/Cargo.toml", "p/shared/x.rs", "p/src/lib.rs"],
        vec![
            (
                "p/src/lib.rs",
                facts(declared, vec![writes(&["elsewhere", "f"])]),
            ),
            ("p/shared/x.rs", facts(Vec::new(), Vec::new())),
        ],
    )
}

fn an_inline_block_has_its_children_in_a_directory_of_its_name() -> Repository {
    let inside = vec![ModDeclaration {
        gate: crate::domain::module_tree::types::Gate::Always,
        around: vec![EnclosingBlock {
            name: Segment("outer".into()),
            path: Section::Absent,
        }],
        name: Segment("leaf".into()),
        body: Body::File,
        path: Section::Absent,
    }];
    let mut declared = vec![ModDeclaration {
        gate: crate::domain::module_tree::types::Gate::Always,
        around: Vec::new(),
        name: Segment("outer".into()),
        body: Body::Inline,
        path: Section::Absent,
    }];
    declared.extend(inside);
    package(
        vec!["p/Cargo.toml", "p/src/lib.rs", "p/src/outer/leaf.rs"],
        vec![
            (
                "p/src/lib.rs",
                facts(declared, vec![writes(&["outer", "leaf", "g"])]),
            ),
            ("p/src/outer/leaf.rs", facts(Vec::new(), Vec::new())),
        ],
    )
}

fn a_module_with_no_file() -> Repository {
    package(
        vec!["p/Cargo.toml", "p/src/lib.rs"],
        vec![("p/src/lib.rs", facts(declares(&["b"]), Vec::new()))],
    )
}

fn a_module_with_two_files() -> Repository {
    package(
        vec![
            "p/Cargo.toml",
            "p/src/b.rs",
            "p/src/b/mod.rs",
            "p/src/lib.rs",
        ],
        vec![
            ("p/src/lib.rs", facts(declares(&["b"]), Vec::new())),
            ("p/src/b.rs", facts(Vec::new(), Vec::new())),
            ("p/src/b/mod.rs", facts(Vec::new(), Vec::new())),
        ],
    )
}

fn a_module_loaded_in_a_cycle() -> Repository {
    let mut declared = declares(&["again"]);
    declared[0].path = Section::Present(WrittenPath("lib.rs".into()));
    package(
        vec!["p/Cargo.toml", "p/src/lib.rs"],
        vec![("p/src/lib.rs", facts(declared, Vec::new()))],
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_module_file_owns_its_directory"),
        word: Word::Succeeds,
        input: || a_module_file_owns_its_directory,
        expect: Expect::Value("crate=p/src/lib.rs net=p/src/net/mod.rs net::tcp=p/src/net/tcp.rs"),
    },
    Case {
        name: CaseName("a_path_attribute_names_the_file_of_the_module"),
        word: Word::Succeeds,
        input: || a_path_attribute_names_the_file,
        expect: Expect::Value("crate=p/src/lib.rs elsewhere=p/shared/x.rs"),
    },
    Case {
        name: CaseName("an_inline_block_has_its_children_in_a_directory_of_its_name"),
        word: Word::Succeeds,
        input: || an_inline_block_has_its_children_in_a_directory_of_its_name,
        expect: Expect::Value(
            "crate=p/src/lib.rs outer=p/src/lib.rs outer::leaf=p/src/outer/leaf.rs",
        ),
    },
    Case {
        name: CaseName("a_module_with_no_file_is_a_gap_in_the_tree"),
        word: Word::Succeeds,
        input: || a_module_with_no_file,
        expect: Expect::Value(
            "crate=p/src/lib.rs | p/src/lib.rs declares module `b` with no source files",
        ),
    },
    Case {
        name: CaseName("a_module_with_two_files_is_a_gap_in_the_tree"),
        word: Word::Succeeds,
        input: || a_module_with_two_files,
        expect: Expect::Value(
            "crate=p/src/lib.rs | p/src/lib.rs declares module `b` with two source files",
        ),
    },
    Case {
        name: CaseName("a_module_loaded_in_a_cycle_is_a_gap_in_the_tree"),
        word: Word::Succeeds,
        input: || a_module_loaded_in_a_cycle,
        expect: Expect::Value(
            "crate=p/src/lib.rs again=p/src/lib.rs | p/src/lib.rs declares module `again` as p/src/lib.rs, which is already being loaded",
        ),
    },
];

fn subject(made: Input) -> Value {
    let made = made();
    let files: BTreeSet<RepoPath> = made
        .files
        .iter()
        .map(|each| RepoPath((*each).into()))
        .collect();
    let facts: BTreeMap<RepoPath, Facts> = made
        .facts
        .into_iter()
        .map(|(path, each)| (RepoPath(path.into()), each))
        .collect();
    let drawn = tree(
        &files,
        &RepoPath("p".into()),
        &RepoPath("p/src/lib.rs".into()),
        &|path| facts.get(path),
    );
    let modules: Vec<String> = drawn
        .modules
        .iter()
        .map(|(module, owners)| {
            let name = if module.is_empty() {
                "crate".to_owned()
            } else {
                module
                    .iter()
                    .map(|each| &**each)
                    .collect::<Vec<_>>()
                    .join("::")
            };
            let owners: Vec<&str> = owners.iter().map(|owner| &*owner.0).collect();
            format!("{name}={}", owners.join(","))
        })
        .collect();
    let gaps: Vec<String> = drawn.gaps.iter().map(ToString::to_string).collect();
    let shown = if gaps.is_empty() {
        modules.join(" ")
    } else {
        format!("{} | {}", modules.join(" "), gaps.join("; "))
    };
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(shown.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
