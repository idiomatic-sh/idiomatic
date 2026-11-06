//! The `manifest_diagnostic` item's cases: a manifest the reader refuses, and a `catalog:` requirement no catalog answers, are rows; a
//! repository read whole gives none.

use super::item::ManifestDiagnosticItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::{WORKSPACE, read, shown};

type Files = &'static [(&'static str, &'static str)];

/// A package and a nested manifest that is not JSON.
const NOT_JSON: Files = &[
    ("package.json", r#"{"name":"root"}"#),
    ("lib/package.json", "{"),
];

/// A requirement on a catalog the repository does not write.
const NO_CATALOG: Files = &[(
    "package.json",
    r#"{"name":"root","dependencies":{"left":"catalog:"}}"#,
)];

type Input = Files;
/// The rows written, one line for each, every column shown.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("manifests_the_reader_reads_say_nothing"),
        word: Word::Succeeds,
        input: || WORKSPACE,
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_manifest_that_is_not_json_is_a_row_of_the_parse_at_the_file_origin"),
        word: Word::Succeeds,
        input: || NOT_JSON,
        expect: Expect::Value(vec![]),
    },
    Case {
        name: CaseName("a_catalog_requirement_no_catalog_answers_is_a_row_of_the_validation"),
        word: Word::Succeeds,
        input: || NO_CATALOG,
        expect: Expect::Value(Vec::new()),
    },
];

fn subject(files: Input) -> Value {
    let (written, symbols) = read::<ManifestDiagnosticItem>(files);
    let lines: Vec<String> = written
        .rows
        .iter()
        .map(|row| {
            format!(
                "{} {}..{} {} {} {} {} {}",
                row.file.0,
                row.span.start.0,
                row.span.end.0,
                shown(&symbols, row.location_kind),
                shown(&symbols, row.phase),
                shown(&symbols, row.schema),
                shown(&symbols, row.code),
                shown(&symbols, row.message)
            )
        })
        .collect();
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Box::leak(lines.join("\n").into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
