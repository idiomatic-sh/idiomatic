//! The `gomod` adapter runs the port's contract, and its own cases: what a `go.mod` and a `go.work` declare, and the manifests it
//! refuses.

use super::adapter::ManifestReadingGomodAdapter;
use crate::cases::{Case, CaseName, Expect, Selection, Word, run};
use crate::domain::modules::types::{Declared, Indirectness, Manifestness};
use crate::outbound::manifest_reading::errors::ManifestReadingError;
use crate::outbound::manifest_reading::fixtures::contract::check;
use crate::outbound::manifest_reading::interface::ManifestReadingPort as _;
use ryo::languages::ManifestText;

/// The kind of manifest and its text.
type Input = (Manifestness, &'static str);
/// What the manifest declares: its module, its requirements, its replacements and the directories it uses, each list joined.
type Value = &'static str;
type Failure = ManifestReadingError;

/// What a manifest declares, as one line.
fn summary(declared: &Declared) -> String {
    let module = declared
        .module
        .as_ref()
        .map_or("-", |module| &*module.0)
        .to_owned();
    let required: Vec<String> = declared
        .requirements
        .iter()
        .map(|each| match each.indirect {
            Indirectness::Direct => each.path.0.to_string(),
            Indirectness::Indirect => format!("{} (indirect)", each.path.0),
        })
        .collect();
    let replaced: Vec<String> = declared
        .replacements
        .iter()
        .map(|each| {
            // A version is shown after its path, where the line writes one.
            let after = |path: &str, version: &str| {
                if version.is_empty() {
                    path.to_owned()
                } else {
                    format!("{path} {version}")
                }
            };
            format!(
                "{} => {}",
                after(&each.from.0, &each.version.0),
                after(&each.to.0, &each.to_version.0)
            )
        })
        .collect();
    let used: Vec<&str> = declared.uses.iter().map(|each| &*each.0).collect();
    format!(
        "module {module}; require {}; replace {}; use {}",
        required.join(", "),
        replaced.join(", "),
        used.join(", ")
    )
}

/// Whether the manifest is refused for a `require` block on line 2 that is not closed.
fn unclosed_require(error: &ManifestReadingError) -> Selection {
    match error {
        ManifestReadingError::Unclosed { line: 2, verb } if verb == "require" => Selection::Matches,
        ManifestReadingError::NoModule
        | ManifestReadingError::Unopened { .. }
        | ManifestReadingError::Unclosed { .. }
        | ManifestReadingError::Incomplete { .. }
        | ManifestReadingError::Unquoted { .. } => Selection::Differs,
    }
}

/// Whether the manifest is refused for a `replace` on line 2 with no arrow.
fn incomplete_replace(error: &ManifestReadingError) -> Selection {
    match error {
        ManifestReadingError::Incomplete { line: 2, verb } if verb == "replace" => {
            Selection::Matches
        }
        ManifestReadingError::NoModule
        | ManifestReadingError::Unopened { .. }
        | ManifestReadingError::Unclosed { .. }
        | ManifestReadingError::Incomplete { .. }
        | ManifestReadingError::Unquoted { .. } => Selection::Differs,
    }
}

/// The rows of what is specific to this adapter.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_go_mod_declares_its_module_requirements_and_replacements_with_their_versions",
        ),
        word: Word::Succeeds,
        input: || {
            (
                Manifestness::Module,
                "module example.com/app // the app\n\ngo 1.24\n\nrequire (\n\texample.com/core v1.2.0\n\tgithub.com/other/lib v1.0.0 // indirect\n)\n\nrequire \"example.com/quoted\" v0.1.0\n\nreplace example.com/core => ../core\nreplace (\n\texample.com/lib v1.0.0 => example.com/fork v1.0.1\n)\n",
            )
        },
        expect: Expect::Value(
            "module example.com/app; require example.com/core, github.com/other/lib (indirect), example.com/quoted; replace example.com/core => ../core, example.com/lib v1.0.0 => example.com/fork v1.0.1; use ",
        ),
    },
    Case {
        name: CaseName("a_quoted_path_is_read_as_go_reads_it_its_escapes_decoded"),
        word: Word::Succeeds,
        input: || {
            (
                Manifestness::Module,
                "module \"example.com/\\u0061pp\"\n\nrequire \"example.com/\\u007a\" v1.0.0\nrequire `example.com/raw\\n` v1.0.0\n",
            )
        },
        expect: Expect::Value(
            "module example.com/app; require example.com/z, example.com/raw\\n; replace ; use ",
        ),
    },
    Case {
        name: CaseName("a_go_work_declares_the_directories_it_uses"),
        word: Word::Succeeds,
        input: || {
            (
                Manifestness::Workspace,
                "go 1.24\n\nuse (\n\t./app\n\t./libs/core // core\n)\nuse ./tools\n",
            )
        },
        expect: Expect::Value("module -; require ; replace ; use ./app, ./libs/core, ./tools"),
    },
    Case {
        name: CaseName("a_go_mod_with_no_module_is_refused"),
        word: Word::Refuses,
        input: || (Manifestness::Module, "go 1.24\n"),
        expect: Expect::Error(ManifestReadingError::NoModule),
    },
    Case {
        name: CaseName("a_block_that_is_not_closed_is_refused"),
        word: Word::Refuses,
        input: || (Manifestness::Module, "module m\nrequire (\n\ta v1\n"),
        expect: Expect::Refusal(unclosed_require),
    },
    Case {
        name: CaseName("a_closing_parenthesis_with_no_block_is_refused"),
        word: Word::Refuses,
        input: || (Manifestness::Module, "module m\n)\n"),
        expect: Expect::Error(ManifestReadingError::Unopened { line: 2 }),
    },
    Case {
        name: CaseName("a_replace_with_no_arrow_is_refused"),
        word: Word::Refuses,
        input: || (Manifestness::Module, "module m\nreplace a v1\n"),
        expect: Expect::Refusal(incomplete_replace),
    },
    Case {
        name: CaseName("a_string_that_is_not_closed_is_refused"),
        word: Word::Refuses,
        input: || (Manifestness::Module, "module \"m\n"),
        expect: Expect::Error(ManifestReadingError::Unquoted { line: 1 }),
    },
];

fn subject((kind, text): Input) -> Result<Value, Failure> {
    ManifestReadingGomodAdapter
        .declared(kind, &ManifestText(text.into()))
        .map(|declared| &*Box::leak(summary(&declared).into_boxed_str()))
}

#[test]
fn succeeds() {
    check(&ManifestReadingGomodAdapter);
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
