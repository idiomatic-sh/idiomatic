//! The `targets` service's cases, on repositories held in memory.

use super::service::targets_of;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::paths::types::RepoPath;
use crate::domain::paths::types::Section;
use crate::domain::targets::types::Build;
use crate::domain::targets::types::Discovery;
use crate::domain::targets::types::LibrarySection;
use crate::domain::targets::types::Manifest;
use crate::domain::targets::types::PackageName;
use crate::domain::targets::types::PackageSection;
use crate::domain::targets::types::TargetName;
use crate::domain::targets::types::TargetSection;
use crate::domain::targets::types::WrittenPath;
use crate::domain::workspaces::fixtures::answering;

/// What a row of the cases does: the files of the package and the manifest it has.
#[derive(Clone, Copy)]
enum Scenario {
    /// A library, a main file, a binary found, an example and a test, all by their place.
    Found,
    /// A binary declared with a path, the tests left out and the build script named.
    Declared,
    /// A library declared where it is not found.
    DeclaredLibrary,
}

/// What a row of the cases does.
type Input = Scenario;
/// The source files, in order.
type Value = &'static str;
type Failure = ();

fn package(name: &str) -> PackageSection {
    PackageSection {
        name: PackageName(name.into()),
        autolib: Discovery::Automatic,
        autobins: Discovery::Automatic,
        autoexamples: Discovery::Automatic,
        autotests: Discovery::Automatic,
        autobenches: Discovery::Automatic,
        ..PackageSection::default()
    }
}

fn subject(scenario: Input) -> Value {
    let (files, manifest, package) = match scenario {
        Scenario::Found => (
            vec![
                "src/lib.rs",
                "src/main.rs",
                "src/bin/extra.rs",
                "src/bin/dir/main.rs",
                "examples/demo.rs",
                "tests/it.rs",
                "build.rs",
            ],
            Manifest::default(),
            package("p"),
        ),
        Scenario::Declared => (
            vec![
                "src/lib.rs",
                "tests/it.rs",
                "gen.rs",
                "tools/tool.rs",
                "src/bin/extra.rs",
            ],
            Manifest {
                bins: vec![TargetSection {
                    name: Section::Present(TargetName("tool".into())),
                    path: Section::Present(WrittenPath("tools/tool.rs".into())),
                }],
                ..Manifest::default()
            },
            PackageSection {
                autotests: Discovery::Declared,
                build: Build::At(WrittenPath("gen.rs".into())),
                ..package("p")
            },
        ),
        Scenario::DeclaredLibrary => (
            vec!["lib/mine.rs"],
            Manifest {
                lib: Section::Present(LibrarySection {
                    path: Section::Present(WrittenPath("./lib/mine.rs".into())),
                }),
                ..Manifest::default()
            },
            package("p"),
        ),
    };
    let held: &'static [(&'static str, &'static str)] = Box::leak(
        files
            .into_iter()
            .map(|path| (path, ""))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let found = answering(held, |looking| {
        targets_of(
            &RepoPath(Box::default()),
            &manifest,
            &package,
            &mut looking.directories,
        )
    });
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(
        found
            .into_iter()
            .map(|path| path.0)
            .collect::<Vec<_>>()
            .join(" ")
            .into_boxed_str(),
    )
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("targets_are_found_by_their_place"),
        word: Word::Succeeds,
        input: || Scenario::Found,
        expect: Expect::Value(
            "build.rs examples/demo.rs src/bin/dir/main.rs src/bin/extra.rs src/lib.rs src/main.rs tests/it.rs",
        ),
    },
    Case {
        name: CaseName("targets_are_declared_by_name_and_left_out_by_the_flags"),
        word: Word::Succeeds,
        input: || Scenario::Declared,
        expect: Expect::Value("gen.rs src/bin/extra.rs src/lib.rs tools/tool.rs"),
    },
    Case {
        name: CaseName("a_library_is_where_it_is_declared_to_be"),
        word: Word::Succeeds,
        input: || Scenario::DeclaredLibrary,
        expect: Expect::Value("lib/mine.rs"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
