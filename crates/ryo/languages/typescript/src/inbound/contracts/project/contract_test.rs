//! Project resolution preserves valid manifests, reports malformed manifests, and links the files only when a relation of the links
//! is asked for.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{WORKSPACE, project, request};
use ryo::languages::{Project as _, contract};

/// Whether the run asks for a relation of the links between the files.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// It asks for `source_dependency`.
    Links,
    /// It asks for none.
    Nothing,
}

/// The files of a repository, each with its path and its text, and what the run asks for.
type Input = (&'static [(&'static str, &'static str)], Asked);
/// How many manifests it names, how many packages it declared, how many it refused and how many files it examined.
type Value = (usize, usize, usize, usize);
type Failure = ryo::languages::ProjectError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("npm_manifests_and_configs"),
        word: Word::Succeeds,
        input: || (WORKSPACE, Asked::Nothing),
        expect: Expect::Value((3, 6, 0, 0)),
    },
    Case {
        name: CaseName("the_files_are_examined_when_a_relation_of_the_links_is_asked_for"),
        word: Word::Succeeds,
        input: || (WORKSPACE, Asked::Links),
        expect: Expect::Value((3, 6, 0, 2)),
    },
    Case {
        name: CaseName("broken_manifest_is_reported"),
        word: Word::Succeeds,
        input: || {
            (
                &[("package.json", "{}"), ("broken/package.json", "{")],
                Asked::Nothing,
            )
        },
        expect: Expect::Value((3, 1, 1, 0)),
    },
];
fn subject((files, asked): Input) -> Result<Value, Failure> {
    let language = project();
    contract::project(&language);
    let wanted = match asked {
        Asked::Links => vec![ryo::languages::Relation::SourceDependency],
        Asked::Nothing => Vec::new(),
    };
    let resolved = language.resolve(&request(files, wanted))?;
    Ok((
        language.manifests().len(),
        resolved.derived.packages.entries.len(),
        resolved.refused.len(),
        resolved.derived.analyzed.len(),
    ))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
