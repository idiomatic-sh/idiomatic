//! Proves the real wiring of `linter lint` over fake connections, as rows of cases: where the walk skips the store, and the
//! engines that run the files of the language of words (the old engine, the new engine over the engine port's fake, or both,
//! compared), each through the real composition, core's lint and the store: a run given what the store holds the pack of
//! compiles and lowers nothing under the new engine alone, and a changed rule, fragment or selection compiles and lowers again.

use super::composition::store_skip;
use super::entry::lint_with;
use super::fixtures::given_words::{GIVEN, REFUSED, given_request};
use super::fixtures::words::{WORDS, WORDS_AGAIN, words_linter, words_request};
use super::types::{Cache, LintReport};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_operators::spike::{EngineMode, engine_mode};
use std::borrow::Cow;
use std::sync::atomic::Ordering;

/// How a row spells where the store is.
#[derive(Clone, Copy, Debug)]
enum Store {
    /// As written.
    Written(&'static str),
    /// As written, made absolute from the current directory.
    Absolute(&'static str),
}

/// Which engines a row's linter runs the files of the language of words with.
#[derive(Clone, Copy, Debug)]
enum Engines {
    /// The ones the row says.
    These(EngineMode),
    /// The ones `RYO_ENGINE` selects for the process: the old engine when it is not set.
    OfTheEnvironment,
}

/// What a row gives the wiring.
#[derive(Clone, Copy, Debug)]
enum Wired {
    /// Where the store is kept and where the root is, as the caller spelled them.
    Skip(Store, &'static str),
    /// One file of two words on two lines, checked once by the rule of the words.
    Words(Engines),
    /// The same file checked three times over one kept store: twice by the same rule, then by the rule under another text.
    Packs(Engines),
    /// The same file checked ten times over one kept store, each run given what the one before it was or one thing changed:
    /// the text of a rule, the text of the fragment the rules share, which rules are selected, which language is named.
    Given(Engines),
    /// The same file checked by two rules, one of which the new engine's load refuses: what each rule found, and what the
    /// report says no rule could decide.
    Refused(Engines),
    /// The same file checked by both engines when the old engine's parse ends each word a byte early.
    Planted,
}

type Input = Wired;
/// What the wiring builds: the skip the walk is given for the store, or `no skip`; the report of a run, in lines; what the
/// engine port's fake was asked over three runs.
type Value = Cow<'static, str>;
/// Why the run ends in error.
type Failure = Cow<'static, str>;

/// The report every engine answers for the file of the two words.
const REPORT: &str = "words Is(File(RelativePath(\"words.walk\"))) FindingKey([Is(Text(HostText(\"words.walk#1\")))]) At { line: Line(1), column: Column(1) } [id=Is(Text(HostText(\"words.walk#1\"))), file=Is(File(RelativePath(\"words.walk\"))), span=Is(Span(Span { start: Offset(0), end: Offset(3) })), text=Is(Text(HostText(\"two\")))]\nwords Is(File(RelativePath(\"words.walk\"))) FindingKey([Is(Text(HostText(\"words.walk#2\")))]) At { line: Line(2), column: Column(2) } [id=Is(Text(HostText(\"words.walk#2\"))), file=Is(File(RelativePath(\"words.walk\"))), span=Is(Span(Span { start: Offset(5), end: Offset(8) })), text=Is(Text(HostText(\"one\")))]\nunread []; unknowns []";

/// The rows of this composition's cases: the store and the root as spelled, and the skip that results.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_store_beneath_the_current_directory_root_is_skipped"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Written(".ryo_cache"), "."),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_and_a_root_spelled_differently_name_the_same_directory"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Written("proj/.ryo_cache"), "./proj"),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_with_a_parent_component_names_the_directory_it_reaches"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Written("proj/x/../.ryo_cache"), "proj/"),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("an_absolute_store_beneath_a_relative_root_is_skipped"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Absolute("proj/.ryo_cache"), "proj"),
        expect: Expect::Value(Cow::Borrowed("./.ryo_cache")),
    },
    Case {
        name: CaseName("a_store_deeper_beneath_the_root_is_skipped_by_its_whole_path"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Written("src/.cache/ryo"), "."),
        expect: Expect::Value(Cow::Borrowed("./src/.cache/ryo")),
    },
    Case {
        name: CaseName("a_store_outside_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Written("elsewhere/cache"), "proj"),
        expect: Expect::Value(Cow::Borrowed("no skip")),
    },
    Case {
        name: CaseName("a_store_above_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Written("../cache"), "."),
        expect: Expect::Value(Cow::Borrowed("no skip")),
    },
    Case {
        name: CaseName("a_store_that_is_the_root_is_skipped_from_nothing"),
        word: Word::Succeeds,
        input: || Wired::Skip(Store::Written("proj"), "proj/"),
        expect: Expect::Value(Cow::Borrowed("no skip")),
    },
    Case {
        name: CaseName("the_old_engine_reports_the_words_of_a_file"),
        word: Word::Succeeds,
        input: || Wired::Words(Engines::These(EngineMode::Old)),
        expect: Expect::Value(Cow::Borrowed(REPORT)),
    },
    Case {
        name: CaseName("the_new_engine_reports_the_words_of_a_file_as_the_old_engine_does"),
        word: Word::Succeeds,
        input: || Wired::Words(Engines::These(EngineMode::New)),
        expect: Expect::Value(Cow::Borrowed(REPORT)),
    },
    Case {
        name: CaseName("both_engines_compared_report_the_words_of_a_file"),
        word: Word::Succeeds,
        input: || Wired::Words(Engines::These(EngineMode::Both)),
        expect: Expect::Value(Cow::Borrowed(REPORT)),
    },
    Case {
        name: CaseName("the_engines_the_environment_selects_report_the_words_of_a_file"),
        word: Word::Succeeds,
        input: || Wired::Words(Engines::OfTheEnvironment),
        expect: Expect::Value(Cow::Borrowed(REPORT)),
    },
    Case {
        name: CaseName("the_old_engine_asks_nothing_of_the_new_one_and_reads_no_pack"),
        word: Word::Succeeds,
        input: || Wired::Packs(Engines::These(EngineMode::Old)),
        expect: Expect::Value(Cow::Borrowed(
            "Old: lowerings 0 packs read 0 findings 2; lowerings 0 packs read 0 findings 2; lowerings 0 packs read 0 findings 2",
        )),
    },
    Case {
        name: CaseName(
            "a_second_run_of_the_new_engine_reads_the_pack_and_a_changed_rule_lowers_again",
        ),
        word: Word::Succeeds,
        input: || Wired::Packs(Engines::These(EngineMode::New)),
        expect: Expect::Value(Cow::Borrowed(
            "New: lowerings 2 packs read 0 findings 2; lowerings 2 packs read 1 findings 2; lowerings 4 packs read 1 findings 2",
        )),
    },
    Case {
        name: CaseName(
            "a_second_run_of_both_engines_reads_the_pack_and_a_changed_rule_lowers_again",
        ),
        word: Word::Succeeds,
        input: || Wired::Packs(Engines::These(EngineMode::Both)),
        expect: Expect::Value(Cow::Borrowed(
            "Both: lowerings 2 packs read 0 findings 2; lowerings 2 packs read 1 findings 2; lowerings 4 packs read 1 findings 2",
        )),
    },
    Case {
        name: CaseName(
            "the_new_engine_given_what_the_store_holds_compiles_nothing_and_each_change_compiles_again",
        ),
        word: Word::Succeeds,
        input: || Wired::Given(Engines::These(EngineMode::New)),
        expect: Expect::Value(Cow::Borrowed(
            "New\nfirst: compiled 1 lowered 2 packs read 0 findings 2\nsame: compiled 0 lowered 0 packs read 1 findings 2\nrule text changed: compiled 1 lowered 2 packs read 0 findings 2\nsame: compiled 0 lowered 0 packs read 1 findings 2\nfragment text changed: compiled 1 lowered 2 packs read 0 findings 2\nsame: compiled 0 lowered 0 packs read 1 findings 2\none more rule selected: compiled 2 lowered 3 packs read 0 findings 4\nsame: compiled 0 lowered 0 packs read 1 findings 4\nthe language named: compiled 2 lowered 3 packs read 0 findings 4\nsame: compiled 0 lowered 0 packs read 1 findings 4",
        )),
    },
    Case {
        name: CaseName(
            "both_engines_given_what_the_store_holds_lower_nothing_and_compile_for_the_old_engine",
        ),
        word: Word::Succeeds,
        input: || Wired::Given(Engines::These(EngineMode::Both)),
        expect: Expect::Value(Cow::Borrowed(
            "Both\nfirst: compiled 1 lowered 2 packs read 0 findings 2\nsame: compiled 1 lowered 0 packs read 1 findings 2\nrule text changed: compiled 1 lowered 2 packs read 0 findings 2\nsame: compiled 1 lowered 0 packs read 1 findings 2\nfragment text changed: compiled 1 lowered 2 packs read 0 findings 2\nsame: compiled 1 lowered 0 packs read 1 findings 2\none more rule selected: compiled 2 lowered 3 packs read 0 findings 4\nsame: compiled 2 lowered 0 packs read 1 findings 4\nthe language named: compiled 2 lowered 3 packs read 0 findings 4\nsame: compiled 2 lowered 0 packs read 1 findings 4",
        )),
    },
    Case {
        name: CaseName("the_old_engine_compiles_every_run_and_reads_no_pack"),
        word: Word::Succeeds,
        input: || Wired::Given(Engines::These(EngineMode::Old)),
        expect: Expect::Value(Cow::Borrowed(
            "Old\nfirst: compiled 1 lowered 0 packs read 0 findings 2\nsame: compiled 1 lowered 0 packs read 0 findings 2\nrule text changed: compiled 1 lowered 0 packs read 0 findings 2\nsame: compiled 1 lowered 0 packs read 0 findings 2\nfragment text changed: compiled 1 lowered 0 packs read 0 findings 2\nsame: compiled 1 lowered 0 packs read 0 findings 2\none more rule selected: compiled 2 lowered 0 packs read 0 findings 4\nsame: compiled 2 lowered 0 packs read 0 findings 4\nthe language named: compiled 2 lowered 0 packs read 0 findings 4\nsame: compiled 2 lowered 0 packs read 0 findings 4",
        )),
    },
    Case {
        name: CaseName("a_rule_the_new_engine_refuses_is_reported_as_not_decided_never_as_run"),
        word: Word::Succeeds,
        input: || Wired::Refused(Engines::These(EngineMode::New)),
        expect: Expect::Value(Cow::Borrowed(
            "kinds found 0; words found 2; unknowns [Unknown { rule: RuleId(\"kinds\"), path: RelativePath(\".\"), message: Message(\"the new engine's rule set of the language `words` does not run the rule: its load refused it, so the rule looked at none of that language's files\") }]",
        )),
    },
    Case {
        name: CaseName(
            "both_engines_report_what_the_old_engine_decided_of_a_rule_the_new_one_refuses",
        ),
        word: Word::Succeeds,
        input: || Wired::Refused(Engines::These(EngineMode::Both)),
        expect: Expect::Value(Cow::Borrowed("kinds found 2; words found 2; unknowns []")),
    },
    Case {
        name: CaseName("the_old_engine_runs_a_rule_the_new_engine_would_refuse"),
        word: Word::Succeeds,
        input: || Wired::Refused(Engines::These(EngineMode::Old)),
        expect: Expect::Value(Cow::Borrowed("kinds found 2; words found 2; unknowns []")),
    },
    Case {
        name: CaseName("a_planted_difference_fails_both_naming_the_rule_the_file_and_the_row"),
        word: Word::Refuses,
        input: || Wired::Planted,
        expect: Expect::Error(Cow::Borrowed(
            "execution: engine harness: rows differ: RowsDiffer { rule: Some(RuleId(\"words\")), file: Some(FilePath(\"words.walk\")), old: Some(ComparedRow(\"words\\twords.walk\\t1\\t#\\tid\\t#\\tfile\\twords.walk\\tspan\\t0..2\\ttext\\ttw\\t1\\t1\")), new: Some(ComparedRow(\"words\\twords.walk\\t1\\t#\\tid\\t#\\tfile\\twords.walk\\tspan\\t0..3\\ttext\\ttwo\\t1\\t1\")) }",
        )),
    },
];

/// A report as lines: each finding with its place and its cells, then what no language read and what no rule could decide.
fn lines(report: &LintReport) -> String {
    let mut lines: Vec<String> = report
        .findings
        .iter()
        .map(|finding| {
            let columns: Vec<String> = finding
                .columns
                .iter()
                .map(|column| format!("{}={:?}", column.name.0, column.value))
                .collect();
            format!(
                "{} {:?} {:?} {:?} [{}]",
                finding.rule.0,
                finding.subject,
                finding.key,
                finding.place,
                columns.join(", ")
            )
        })
        .collect();
    lines.push(format!(
        "unread {:?}; unknowns {:?}",
        report.unread, report.unknowns
    ));
    lines.join("\n")
}

/// The engines a row runs, as the fixture linter takes them.
const fn said(engines: Engines) -> Option<EngineMode> {
    match engines {
        Engines::These(mode) => Some(mode),
        Engines::OfTheEnvironment => None,
    }
}

/// One file of two words on two lines, in a new root.
fn words_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a root");
    std::fs::write(root.path().join("words.walk"), "two\n one\n").expect("the file of words");
    root
}

/// Calls the real wiring over one row.
fn subject(input: Input) -> Result<Value, Failure> {
    let refused = |error: super::errors::LintError| Cow::Owned(error.to_string());
    match input {
        Wired::Skip(store, root) => {
            let store = match store {
                Store::Written(text) => std::path::PathBuf::from(text),
                Store::Absolute(text) => {
                    std::path::absolute(text).map_err(|error| Cow::Owned(error.to_string()))?
                }
            };
            Ok(store_skip(&store, std::path::Path::new(root))
                .and_then(|skip| skip.to_str().map(|text| Cow::Owned(text.to_owned())))
                .unwrap_or(Cow::Borrowed("no skip")))
        }
        Wired::Words(engines) => {
            let root = words_root();
            let (linter, _fake, _compiler) = words_linter(said(engines), Cache::None, 0);
            let report = lint_with(&linter, words_request(root.path(), WORDS)).map_err(refused)?;
            Ok(Cow::Owned(lines(&report)))
        }
        Wired::Packs(engines) => {
            let (root, store) = (words_root(), tempfile::tempdir().expect("a store"));
            let (linter, fake, _compiler) =
                words_linter(said(engines), Cache::At(store.path().to_path_buf()), 0);
            let mut asked = Vec::new();
            for query in [WORDS, WORDS, WORDS_AGAIN] {
                let report =
                    lint_with(&linter, words_request(root.path(), query)).map_err(refused)?;
                asked.push(format!(
                    "lowerings {} packs read {} findings {}",
                    fake.lowered.load(Ordering::Relaxed),
                    fake.unpacked.load(Ordering::Relaxed),
                    report.findings.len()
                ));
            }
            // What the fake was asked depends on the engines that ran: the row names them.
            let ran = said(engines).unwrap_or_else(engine_mode);
            Ok(Cow::Owned(format!("{ran:?}: {}", asked.join("; "))))
        }
        Wired::Given(engines) => {
            let (root, store) = (words_root(), tempfile::tempdir().expect("a store"));
            let (linter, fake, compiler) =
                words_linter(said(engines), Cache::At(store.path().to_path_buf()), 0);
            let asked = || {
                [
                    compiler.compiled.load(Ordering::Relaxed),
                    fake.lowered.load(Ordering::Relaxed),
                    fake.unpacked.load(Ordering::Relaxed),
                ]
            };
            let mut runs = Vec::new();
            for (what, given) in GIVEN {
                let before = asked();
                let report =
                    lint_with(&linter, given_request(root.path(), *given)).map_err(refused)?;
                let [compiled, lowered, unpacked] = asked();
                runs.push(format!(
                    "{what}: compiled {} lowered {} packs read {} findings {}",
                    compiled - before[0],
                    lowered - before[1],
                    unpacked - before[2],
                    report.findings.len()
                ));
            }
            let ran = said(engines).unwrap_or_else(engine_mode);
            Ok(Cow::Owned(format!("{ran:?}\n{}", runs.join("\n"))))
        }
        Wired::Refused(engines) => {
            let root = words_root();
            let (linter, _fake, _compiler) = words_linter(said(engines), Cache::None, 0);
            let report =
                lint_with(&linter, given_request(root.path(), REFUSED)).map_err(refused)?;
            let found = |rule: &str| {
                report
                    .findings
                    .iter()
                    .filter(|finding| &*finding.rule.0 == rule)
                    .count()
            };
            Ok(Cow::Owned(format!(
                "kinds found {}; words found {}; unknowns {:?}",
                found("kinds"),
                found("words"),
                report.unknowns
            )))
        }
        Wired::Planted => {
            let root = words_root();
            let (linter, _fake, _compiler) = words_linter(Some(EngineMode::Both), Cache::None, 1);
            let report = lint_with(&linter, words_request(root.path(), WORDS)).map_err(refused)?;
            Ok(Cow::Owned(lines(&report)))
        }
    }
}

#[test]
fn succeeds() {
    assert!(!CASES.is_empty(), "the composition has cases");
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
