//! The `links` service's cases: where an import leads as Python's import system finds it, the module before a name, the package's own
//! directories before the others', and a namespace package or a module outside the repository is no file and no doubt.

use super::service::{outcome, python_directories, roots_for};
use super::types::{FilePlace, ImportOutcome, Listing};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{DottedPath, Form, ImportNote, Level, Spelling};
use crate::domain::packages::types::PackageLocation;
use ryo::languages::{FileId, FilePath, Offset};

/// The files of the repository the cases import in, in the order of their paths, and the directories of its packages.
const FILES: &[&str] = &[
    "a/__init__.py",
    "a/b.py",
    "a/c.pyi",
    "lib/pyproject.toml",
    "lib/src/m/__init__.py",
    "lib/src/m/n.py",
    "ns/p.py",
    "pyproject.toml",
    "x.py",
];

/// The directories of the packages: the root's and `lib`'s.
const PACKAGES: &[&str] = &["", "lib"];

/// An import written in a file: the file, the form, the dots and the path.
type Input = (&'static str, Form, u32, &'static str);
type Value = ImportOutcome;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_package_is_its_init"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Module, 0, "a"),
        expect: Expect::Value(ImportOutcome::File(FileId(0))),
    },
    Case {
        name: CaseName("a_module_is_its_file"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Module, 0, "a.b"),
        expect: Expect::Value(ImportOutcome::File(FileId(1))),
    },
    Case {
        name: CaseName("a_module_with_a_stub_alone_is_its_stub"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Module, 0, "a.c"),
        expect: Expect::Value(ImportOutcome::File(FileId(2))),
    },
    Case {
        name: CaseName("a_name_a_package_defines_is_its_init"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Name, 0, "a.thing"),
        expect: Expect::Value(ImportOutcome::File(FileId(0))),
    },
    Case {
        name: CaseName("a_package_in_the_src_layout_of_another_package_is_found"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Name, 0, "m.n"),
        expect: Expect::Value(ImportOutcome::File(FileId(5))),
    },
    Case {
        name: CaseName("a_relative_import_starts_from_the_file_s_package"),
        word: Word::Succeeds,
        input: || ("lib/src/m/n.py", Form::Name, 1, "thing"),
        expect: Expect::Value(ImportOutcome::File(FileId(4))),
    },
    Case {
        name: CaseName("a_relative_import_above_the_root_is_unresolved"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Name, 3, "thing"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("a_namespace_package_is_no_file"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Module, 0, "ns"),
        expect: Expect::Value(ImportOutcome::Namespace),
    },
    Case {
        name: CaseName("a_module_of_a_namespace_package_is_its_file"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Everything, 0, "ns.p"),
        expect: Expect::Value(ImportOutcome::File(FileId(6))),
    },
    Case {
        name: CaseName("a_missing_module_of_a_package_of_the_repository_is_unresolved"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Module, 0, "a.missing"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("a_module_no_package_of_the_repository_names_is_outside_it"),
        word: Word::Succeeds,
        input: || ("x.py", Form::Module, 0, "requests.adapters"),
        expect: Expect::Value(ImportOutcome::Outside),
    },
];

fn subject((path, form, level, dotted): Input) -> Value {
    let files: Vec<FilePath> = FILES.iter().map(|each| FilePath((*each).into())).collect();
    let directories = python_directories(&files);
    let listing = Listing {
        files: &files,
        directories: &directories,
    };
    let packages: Vec<PackageLocation> = PACKAGES
        .iter()
        .map(|each| PackageLocation((*each).into()))
        .collect();
    let roots = roots_for(listing, &packages, FilePlace(path));
    outcome(
        listing,
        &roots,
        FilePlace(path),
        &ImportNote {
            form,
            level: Level(level),
            path: DottedPath(dotted.into()),
            start: Offset(0),
            end: Offset(1),
            text: Spelling(dotted.into()),
        },
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
