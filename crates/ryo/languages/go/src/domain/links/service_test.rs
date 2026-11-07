//! The `links` service's cases: where an import path leads as `go build` finds it in module mode: a path under the file's module is
//! every Go file of its directory but the tests, the module's own path is its root package, and the standard library, another module
//! and a module nested in the file's are outside the links; a path under the module that names no directory of Go files is unresolved,
//! and a file whose module's `go.mod` was refused is not examined.

use super::service::linked;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::{ImportNote, ImportPath, Note, Spelling};
use crate::domain::modules::types::{
    Declared, Manifest, ManifestPath, ModulePath, Modules, Reason, Refusal,
};
use ryo::languages::{FilePath, Offset};

/// The files of the repository the cases import in, in the order of their paths.
const FILES: &[&str] = &[
    "broken/b.go",
    "broken/go.mod",
    "go.mod",
    "internal/store/sql.go",
    "internal/store/store.go",
    "internal/store/store_test.go",
    "main.go",
    "nested/go.mod",
    "nested/lib/lib.go",
    "testsonly/only_test.go",
];

/// An import written in a file: the file, then the path.
type Input = (&'static str, &'static str);
/// Where it leads: the files it links to, `outside`, `unresolved` or `unread`.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_path_under_the_module_is_every_go_file_of_its_directory_but_the_tests"),
        word: Word::Succeeds,
        input: || ("main.go", "example.com/app/internal/store"),
        expect: Expect::Value("internal/store/sql.go internal/store/store.go"),
    },
    Case {
        name: CaseName("the_module_s_own_path_is_its_root_package"),
        word: Word::Succeeds,
        input: || ("internal/store/sql.go", "example.com/app"),
        expect: Expect::Value("main.go"),
    },
    Case {
        name: CaseName("a_test_of_a_package_links_to_the_other_files_of_it"),
        word: Word::Succeeds,
        input: || {
            (
                "internal/store/store_test.go",
                "example.com/app/internal/store",
            )
        },
        expect: Expect::Value("internal/store/sql.go internal/store/store.go"),
    },
    Case {
        name: CaseName("the_standard_library_is_outside_the_links"),
        word: Word::Succeeds,
        input: || ("main.go", "fmt"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName("another_module_is_outside_the_links"),
        word: Word::Succeeds,
        input: || ("main.go", "github.com/other/lib"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName(
            "a_path_that_starts_with_the_module_s_text_but_not_its_parts_is_another_module",
        ),
        word: Word::Succeeds,
        input: || ("main.go", "example.com/apple"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName("a_directory_of_a_module_nested_in_the_file_s_is_another_module"),
        word: Word::Succeeds,
        input: || ("main.go", "example.com/app/nested/lib"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName("a_file_of_a_nested_module_reads_its_own_module"),
        word: Word::Succeeds,
        input: || ("nested/lib/lib.go", "example.com/app/internal/store"),
        expect: Expect::Value("outside"),
    },
    Case {
        name: CaseName("a_path_under_the_module_that_names_no_directory_is_unresolved"),
        word: Word::Succeeds,
        input: || ("main.go", "example.com/app/missing"),
        expect: Expect::Value("unresolved"),
    },
    Case {
        name: CaseName("a_directory_of_tests_alone_is_no_package_and_is_unresolved"),
        word: Word::Succeeds,
        input: || ("main.go", "example.com/app/testsonly"),
        expect: Expect::Value("unresolved"),
    },
    Case {
        name: CaseName("a_file_whose_module_s_go_mod_was_refused_is_not_examined"),
        word: Word::Succeeds,
        input: || ("broken/b.go", "example.com/app/internal/store"),
        expect: Expect::Value("unread"),
    },
];

/// The modules: `example.com/app` at the root, `example.com/app/nested` in `nested/`, and a refused `go.mod` in `broken/`.
fn modules() -> Modules {
    let module = |manifest: &str, path: &str| Manifest {
        manifest: ManifestPath(manifest.into()),
        declared: Declared {
            module: Some(ModulePath(path.into())),
            ..Declared::default()
        },
    };
    Modules {
        manifests: vec![
            module("go.mod", "example.com/app"),
            module("nested/go.mod", "example.com/app/nested"),
        ],
        refused: vec![Refusal {
            manifest: ManifestPath("broken/go.mod".into()),
            reason: Reason("no module".into()),
        }],
    }
}

fn subject((file, path): Input) -> Value {
    let files: Vec<FilePath> = FILES.iter().map(|each| FilePath((*each).into())).collect();
    let notes: Vec<Option<Note>> = FILES
        .iter()
        .map(|each| {
            (*each == file).then(|| Note {
                imports: vec![ImportNote {
                    path: ImportPath(path.into()),
                    start: Offset(0),
                    end: Offset(1),
                    text: Spelling(path.into()),
                }],
            })
        })
        .collect();
    let found = linked(&files, &notes, modules());
    let shown = if !found.dependencies.is_empty() {
        found
            .dependencies
            .iter()
            .filter_map(|link| {
                let at = usize::try_from(link.target.0).ok()?;
                FILES.get(at)
            })
            .copied()
            .collect::<Vec<_>>()
            .join(" ")
    } else if found.analyzed.is_empty() {
        "unread".to_owned()
    } else if found.unresolved.is_empty() {
        "outside".to_owned()
    } else {
        "unresolved".to_owned()
    };
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(shown.into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
