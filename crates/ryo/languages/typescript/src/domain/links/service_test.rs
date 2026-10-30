//! The `links` service's cases: where a specifier leads as TypeScript's resolution finds it, its file before its endings, its endings
//! before its `index`, an alias of `paths` before `baseUrl`, and a package of the repository or outside it is no file and no doubt.

use super::service::outcome;
use super::types::ImportOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::Specifier;
use crate::domain::packages::types::{
    AliasPattern, Declared, ManifestPath, Package, Packages, PathAlias, RepositoryPath,
};
use ryo::languages::{FileId, FilePath};

/// The files of the repository the cases import in, in the order of their paths.
const FILES: &[&str] = &[
    "base/tsconfig.json",
    "lib/b.ts",
    "lib/c/index.tsx",
    "src/a.ts",
    "src/a.vue",
    "src/m.mts",
    "src/tsconfig.json",
    "src/x/index.d.ts",
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
        input: || ("src/x/y.ts", "../a"),
        expect: Expect::Value(ImportOutcome::File(FileId(3))),
    },
    Case {
        name: CaseName("a_specifier_that_names_a_file_is_it"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "./a.vue"),
        expect: Expect::Value(ImportOutcome::File(FileId(4))),
    },
    Case {
        name: CaseName("a_javascript_ending_names_the_typescript_file"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "./m.mjs"),
        expect: Expect::Value(ImportOutcome::File(FileId(5))),
    },
    Case {
        name: CaseName("a_directory_is_its_index"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "./x"),
        expect: Expect::Value(ImportOutcome::File(FileId(7))),
    },
    Case {
        name: CaseName("an_alias_of_an_extended_configuration_holds"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "@lib/c"),
        expect: Expect::Value(ImportOutcome::File(FileId(2))),
    },
    Case {
        name: CaseName("a_path_from_the_base_url_is_a_file"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "lib/b"),
        expect: Expect::Value(ImportOutcome::File(FileId(1))),
    },
    Case {
        name: CaseName("an_alias_that_maps_to_no_file_is_unresolved"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "@lib/none"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("a_relative_specifier_that_names_no_file_is_unresolved"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "./none"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("a_relative_specifier_above_the_root_is_unresolved"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "../../a"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("an_import_of_the_package_s_own_imports_is_unresolved"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "#internal"),
        expect: Expect::Value(ImportOutcome::Unresolved),
    },
    Case {
        name: CaseName("a_package_is_outside_the_links"),
        word: Word::Succeeds,
        input: || ("src/b.ts", "react"),
        expect: Expect::Value(ImportOutcome::Outside),
    },
    Case {
        name: CaseName("a_file_no_configuration_holds_reads_no_alias"),
        word: Word::Succeeds,
        input: || ("b.ts", "@lib/c"),
        expect: Expect::Value(ImportOutcome::Outside),
    },
];

/// The configurations: `src/tsconfig.json` extends `base/tsconfig.json`, whose `paths` alias `@lib/*` and whose `baseUrl` is the root.
fn packages() -> Packages {
    Packages {
        entries: vec![
            Package {
                manifest: ManifestPath("base/tsconfig.json".into()),
                declared: Declared {
                    base_url: Some(ManifestPath("".into())),
                    aliases: vec![PathAlias {
                        pattern: AliasPattern("@lib/*".into()),
                        targets: vec![ManifestPath("lib/*".into())],
                    }],
                    ..Declared::default()
                },
            },
            Package {
                manifest: ManifestPath("src/tsconfig.json".into()),
                declared: Declared {
                    extends: vec![ManifestPath("base/tsconfig.json".into())],
                    ..Declared::default()
                },
            },
        ],
        refused: Vec::new(),
    }
}

fn subject((path, specifier): Input) -> Value {
    let files: Vec<FilePath> = FILES.iter().map(|each| FilePath((*each).into())).collect();
    outcome(
        &files,
        &packages(),
        RepositoryPath(path),
        &Specifier(specifier.into()),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
