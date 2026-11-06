//! The `repository` service's cases, on values alone.

use super::service::{by_path, configuration, paths_of};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusEntry, CorpusInput, FileSize, Nanoseconds};
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::rule_requests::types::RuleParameters;
use ryo_values::domain::rule_requests::types::RuleSpec;
use ryo_values::domain::runs::types::RuleId;

/// What a row of the cases does with the files of a run.
#[derive(Clone, Copy)]
enum Scenario {
    /// Puts three files out of order in the order of their paths and reads their sizes.
    SortedByPath,
    /// Reads the paths of three files.
    Paths,
    /// Digests the same configuration twice, then with another rule, then with one more rule.
    Configured,
}

/// What a row of the cases does with the files of a run.
type Input = Scenario;
/// Three numbers: a file stands for its size or its number, a path for its first byte; -1 says there was nothing.
type Value = [i64; 3];
type Failure = ();

fn entry(path: &str, size: u64) -> CorpusEntry {
    CorpusEntry {
        executable: Default::default(),
        path: FilePath(path.into()),
        size: FileSize(size),
        modified: Nanoseconds(0),
    }
}

fn files() -> Vec<CorpusInput> {
    vec![entry("c", 3), entry("a", 1), entry("b", 2)]
        .into_iter()
        .map(CorpusInput::Available)
        .collect()
}

fn rule(id: &str, query: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(query.into())),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

fn files_in_order() -> Vec<CorpusInput> {
    let mut files = files();
    by_path(&mut files);
    files
}

fn subject(scenario: Input) -> Value {
    match scenario {
        Scenario::SortedByPath => {
            let mut sorted = [0; 3];
            for (slot, file) in sorted.iter_mut().zip(files_in_order()) {
                let CorpusInput::Available(file) = file else {
                    panic!("fixture requires observed metadata");
                };
                *slot = i64::try_from(file.size.0).unwrap_or(i64::MAX);
            }
            sorted
        }
        Scenario::Paths => {
            let mut read = [0; 3];
            for (slot, path) in read.iter_mut().zip(paths_of(&files())) {
                *slot = i64::from(path.0.bytes().next().unwrap_or(0));
            }
            read
        }
        Scenario::Configured => {
            let digest = |rules: &[RuleSpec]| configuration(rules, &[]);
            let first = digest(&[rule("a", "select x")]);
            let again = digest(&[rule("a", "select x")]);
            let other_rule = digest(&[rule("a", "select y")]);
            let one_more = digest(&[rule("a", "select x"), rule("b", "select x")]);
            [
                i64::from(first == again),
                i64::from(first == other_rule),
                i64::from(first == one_more),
            ]
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("files_are_put_in_the_order_of_their_paths"),
        word: Word::Succeeds,
        input: || Scenario::SortedByPath,
        expect: Expect::Value([1, 2, 3]),
    },
    Case {
        name: CaseName("the_paths_are_read_in_the_order_of_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Paths,
        expect: Expect::Value([99, 97, 98]),
    },
    Case {
        name: CaseName(
            "the_same_configuration_has_the_same_bytes_and_another_rule_or_one_more_other_bytes",
        ),
        word: Word::Succeeds,
        input: || Scenario::Configured,
        expect: Expect::Value([1, 0, 0]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

#[test]
fn explicit_source_selection_identity_is_exact_and_framed() {
    use ryo_values::domain::descriptions::types::LanguageName;
    let encode = super::service::source_selection_identity;
    let python = LanguageName("python".into());
    let other = LanguageName("python-other".into());
    assert_eq!(encode(Some(&python)), encode(Some(&python)));
    assert_ne!(encode(None), encode(Some(&python)));
    assert_ne!(encode(Some(&python)), encode(Some(&other)));
    assert_ne!(encode(None), encode(Some(&LanguageName("".into()))));
}
