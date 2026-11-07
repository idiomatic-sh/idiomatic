//! The `links` service's cases: where a specifier leads as Node's resolution finds a relative one, its file before its endings, its
//! endings before its `index`; a package of the repository or outside it is no file and no doubt.

use super::service::outcome;
use super::types::ImportOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::Specifier;
use crate::domain::packages::types::RepositoryPath;
use ryo::languages::{FileId, FilePath};

/// The files of the repository the cases import in, in the order of their paths.
const FILES: &[&str] = &[
    "lib/b.cjs",
    "lib/c/index.jsx",
    "src/a.js",
    "src/a.js.js",
    "src/a.vue",
    "src/m.mjs",
    "src/x/index.js",
    "src/x/index.mjs",
];

/// A specifier written in a file: the file, then the specifier.
type Input = (&'static str, &'static str);
type Value = ImportOutcome;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relative_specifier_is_tried_with_each_ending"),
        word: Word::Succeeds,
        input: || ("src/x/y.js", "../a"),
        expect: Expect::Value(ImportOutcome::File(FileId(2))),
    },
    Case {
        name: CaseName("a_specifier_that_names_a_file_is_it_before_its_endings"),
        word: Word::Succeeds,
        input: || ("src/b.js", "./a.js"),
        expect: Expect::Value(ImportOutcome::File(FileId(2))),
    },
    Case {
        name: CaseName("a_specifier_that_names_a_file_of_another_language_is_it"),
        word: Word::Succeeds,
        input: || ("src/b.js", "./a.vue"),
        expect: Expect::Value(ImportOutcome::File(FileId(4))),
    },
    Case {
        name: CaseName("a_module_ending_is_tried"),
        word: Word::Succeeds,
        input: || ("src/b.js", "./m"),
        expect: Expect::Value(ImportOutcome::File(FileId(5))),
    },
    Case {
        name: CaseName("a_commonjs_ending_is_tried"),
        word: Word::Succeeds,
        input: || ("src/b.js", "../lib/b"),
        expect: Expect::Value(ImportOutcome::File(FileId(0))),
    },
    Case {
        name: CaseName("a_directory_is_its_index_with_the_first_ending_that_names_a_file"),
        word: Word::Succeeds,
        input: || ("src/b.js", "./x"),
        expect: Expect::Value(ImportOutcome::File(FileId(6))),
    },
    Case {
        name: CaseName("a_directory_s_index_is_tried_with_each_ending"),
        word: Word::Succeeds,
        input: || ("src/b.js", "../lib/c"),
        expect: Expect::Value(ImportOutcome::File(FileId(1))),
    },
    Case {
        name: CaseName("a_relative_specifier_that_names_no_file_is_unresolved"),
        word: Word::Succeeds,
        input: || ("src/b.js", "./none"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("a_relative_specifier_above_the_root_is_unresolved"),
        word: Word::Succeeds,
        input: || ("src/b.js", "../../a"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("an_import_of_the_package_s_own_imports_is_unresolved"),
        word: Word::Succeeds,
        input: || ("src/b.js", "#internal"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("a_package_is_outside_the_links"),
        word: Word::Succeeds,
        input: || ("src/b.js", "react"),
        expect: Expect::Value(ImportOutcome::Outside),
    },
    Case {
        name: CaseName("a_module_of_node_is_outside_the_links"),
        word: Word::Succeeds,
        input: || ("src/b.js", "node:fs"),
        expect: Expect::Value(ImportOutcome::Outside),
    },
];

fn subject((path, specifier): Input) -> Value {
    let files: Vec<FilePath> = FILES.iter().map(|each| FilePath((*each).into())).collect();
    outcome(&files, RepositoryPath(path), &Specifier(specifier.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
