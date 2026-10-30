//! What the `project` contract promises: it names `pyproject.toml`, reads the packages the manifests declare through the port,
//! refuses a manifest the tool cannot read without stopping the run, and links the files only when a relation of the links is asked
//! for.

use std::sync::Arc;

use super::contract::ProjectContract;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::language::service::reads;
use crate::fixtures::projects::notes_of;
use crate::outbound::manifest_reading::fixtures::fake::ManifestReadingFake;
use crate::outbound::manifest_reading::interface::ManifestReadingPort;
use crate::outbound::notes::factory::postcard;
use ryo::languages::{self, Project as _};

/// Whether the run asks for a relation of the links between the files.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// It asks for `source_dependency`.
    Links,
    /// It asks for none.
    Nothing,
}

/// The files a repository holds, each with its path and its text, and what the run asks for.
type Input = (&'static [(&'static str, &'static str)], Asked);
/// How many manifests it names, how many packages the manifests declared, how many it refused and how many files it examined.
type Value = (usize, usize, usize, usize);
type Failure = languages::ProjectError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("names_one_manifest_and_a_repository_without_one_gives_no_package"),
        word: Word::Succeeds,
        input: || (&[], Asked::Nothing),
        expect: Expect::Value((1, 0, 0, 0)),
    },
    Case {
        name: CaseName("a_manifest_that_names_a_package_declares_one"),
        word: Word::Succeeds,
        input: || {
            (
                &[("pyproject.toml", "[project]\nname = \"demo\"\n")],
                Asked::Nothing,
            )
        },
        expect: Expect::Value((1, 1, 0, 0)),
    },
    Case {
        name: CaseName("a_manifest_the_tool_cannot_read_is_refused_and_the_run_goes_on"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    ("a/pyproject.toml", "[project\nname"),
                    ("pyproject.toml", "[project]\nname = \"demo\"\n"),
                ],
                Asked::Nothing,
            )
        },
        expect: Expect::Value((1, 1, 1, 0)),
    },
    Case {
        name: CaseName("a_file_that_is_not_a_manifest_is_not_read"),
        word: Word::Succeeds,
        input: || {
            (
                &[("README.md", "[project]\nname = \"demo\"\n")],
                Asked::Nothing,
            )
        },
        expect: Expect::Value((1, 0, 0, 0)),
    },
    Case {
        name: CaseName("the_python_files_are_examined_when_a_relation_of_the_links_is_asked_for"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    ("a.py", ""),
                    ("pyproject.toml", "[project]\nname = \"demo\"\n"),
                ],
                Asked::Links,
            )
        },
        expect: Expect::Value((1, 1, 0, 1)),
    },
    Case {
        name: CaseName("the_files_are_not_linked_when_no_relation_of_the_links_is_asked_for"),
        word: Word::Succeeds,
        input: || {
            (
                &[
                    ("a.py", ""),
                    ("pyproject.toml", "[project]\nname = \"demo\"\n"),
                ],
                Asked::Nothing,
            )
        },
        expect: Expect::Value((1, 1, 0, 0)),
    },
];

/// The adapters of a run: the fake alone.
fn faked(_available: &[languages::ManifestFile]) -> Vec<Arc<dyn ManifestReadingPort>> {
    vec![Arc::new(ManifestReadingFake)]
}

fn subject((files, asked): Input) -> Result<Value, Failure> {
    let contract = ProjectContract::new(faked, Arc::new(postcard()));
    languages::contract::project(&contract);
    let resolved = contract.resolve(&languages::ProjectRequest {
        repository: languages::Repository::Disk,
        root: languages::RootPath("".into()),
        manifests: files
            .iter()
            .filter(|(path, _)| {
                reads(languages::SourcePath(path)) == languages::Recognition::Declines
            })
            .map(|(path, text)| languages::ManifestFile {
                path: languages::FilePath((*path).into()),
                text: languages::ManifestText((*text).into()),
            })
            .collect(),
        files: files
            .iter()
            .map(|(path, _)| languages::FilePath((*path).into()))
            .collect(),
        notes: files
            .iter()
            .map(|(path, text)| notes_of(path, text))
            .collect(),
        wanted: match asked {
            Asked::Links => vec![languages::Relation::SourceDependency],
            Asked::Nothing => Vec::new(),
        },
    })?;
    Ok((
        contract.manifests().len(),
        resolved.derived.packages.packages.len(),
        resolved.refused.len(),
        resolved.derived.analyzed.len(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
