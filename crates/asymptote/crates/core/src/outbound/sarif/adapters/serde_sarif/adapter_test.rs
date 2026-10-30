//! The serde-sarif adapter of the sarif port: the contract, then the real logs of Tastemaker and clippy-sarif, a
//! text that is no log, a result no local file holds, results told apart by where they end, and a region out of
//! range.

use super::adapter::SarifSerdeSarifAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::judgement::types::Inventory;
use crate::domain::violations::types::{Fingerprinted, Level, Place, Suppression};
use crate::outbound::sarif::errors::SarifError;
use crate::outbound::sarif::fixtures::contract::check;
use crate::outbound::sarif::interface::SarifPort as _;
use crate::outbound::sarif::types::SarifText;

/// Tastemaker's `check --format sarif` over ryo: two results, a suppressed one and one with no region.
const TASTEMAKER: &str = include_str!("../../../../../tests/fixtures/sarif/tastemaker.sarif");
/// `cargo clippy --message-format=json | clippy-sarif` over a crate with four warnings.
const CLIPPY: &str = include_str!("../../../../../tests/fixtures/sarif/clippy.sarif");

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract.
    Contract,
    /// Tastemaker's log.
    Tastemaker,
    /// clippy-sarif's log.
    Clippy,
    /// A text that is no log.
    NoLog,
    /// clippy's log with a result at a remote URI.
    Remote,
    /// Two results the same but for where they end: twice.
    EndsDiffer,
    /// Two results the same to their end: once.
    Same,
    /// A result whose region starts on line -1.
    NegativeLine,
    /// A declared rule that did not apply is not evidence for draining its old records.
    NotApplicable,
    /// A log whose driver lists no rule.
    EmptyRules,
    /// A valid error result with no location at all.
    Locationless,
    /// A valid error result with only a logical location.
    LogicalOnly,
    /// A valid error result that names no rule.
    NoRule,
    /// A result whose region starts on line 0, which SARIF's one-based lines cannot hold.
    ZeroLine,
    /// A log that says it is not SARIF 2.1.0.
    OtherVersion,
}

/// A log of one run whose results are at `regions`, each a SARIF region, in `src/lib.rs` under the rule `r`.
fn log(regions: &[&str]) -> String {
    let results: Vec<String> = regions
        .iter()
        .map(|region| {
            format!(
                r#"{{"ruleId": "r", "level": "warning", "message": {{"text": "m"}}, "locations": [{{"physicalLocation": {{"artifactLocation": {{"uri": "src/lib.rs"}}, "region": {region}}}}}]}}"#
            )
        })
        .collect();
    format!(
        r#"{{"version": "2.1.0", "runs": [{{"tool": {{"driver": {{"name": "t"}}}}, "results": [{}]}}]}}"#,
        results.join(", ")
    )
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = SarifError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_that_did_not_apply_is_not_declared_evaluated"),
        word: Word::Succeeds,
        input: || Question::NotApplicable,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_log_whose_driver_lists_no_rule_declares_none"),
        word: Word::Succeeds,
        input: || Question::EmptyRules,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("tastemaker_paths_resolve_its_fingerprints_and_ignores_are_kept"),
        word: Word::Succeeds,
        input: || Question::Tastemaker,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("clippy_sarif_paths_are_relative_and_its_rules_the_ones_it_reported"),
        word: Word::Succeeds,
        input: || Question::Clippy,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("results_that_differ_only_in_where_they_end_count_twice"),
        word: Word::Succeeds,
        input: || Question::EndsDiffer,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("results_the_same_to_their_end_count_once"),
        word: Word::Succeeds,
        input: || Question::Same,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_region_out_of_range_is_refused_so_the_run_fails"),
        word: Word::Refuses,
        input: || Question::NegativeLine,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unlocatable {
                    rule: "r".into(),
                    reason: "its region's startLine -1 is out of range".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_text_that_is_no_log_is_refused"),
        word: Word::Refuses,
        input: || Question::NoLog,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unparsable {
                    reason: "invalid type: integer `1`, expected a sequence at line 1 column 10"
                        .into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_error_with_no_location_is_refused_never_dropped"),
        word: Word::Refuses,
        input: || Question::Locationless,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unlocatable {
                    rule: "GLOBAL".into(),
                    reason: "it has no physical location".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_error_with_only_a_logical_location_is_refused_never_dropped"),
        word: Word::Refuses,
        input: || Question::LogicalOnly,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unlocatable {
                    rule: "GLOBAL".into(),
                    reason: "it has no physical location".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_error_that_names_no_rule_is_refused_never_dropped"),
        word: Word::Refuses,
        input: || Question::NoRule,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unparsable {
                    reason: "a result names no rule".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_region_on_line_zero_is_refused_so_it_is_never_printed"),
        word: Word::Refuses,
        input: || Question::ZeroLine,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unlocatable {
                    rule: "r".into(),
                    reason: "its region's startLine 0 is out of range".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_log_of_another_version_is_refused"),
        word: Word::Refuses,
        input: || Question::OtherVersion,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unparsable {
                    reason: "version \"NOT SARIF\" is not 2.1.0".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_result_no_local_file_holds_is_refused"),
        word: Word::Refuses,
        input: || Question::Remote,
        expect: Expect::Refusal(|actual| {
            if actual
                == &(SarifError::Unlocatable {
                    rule: "unused_variables".into(),
                    reason: "the uri names no local file".into(),
                })
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
];

/// Answers one row's question about the `serde-sarif` adapter.
fn subject(question: Input) -> Result<Value, Failure> {
    let read = |text: &str| SarifSerdeSarifAdapter.read(&SarifText(text.into()));
    let answer = match question {
        Question::Contract => {
            check(&SarifSerdeSarifAdapter);
            Ok(true)
        }
        Question::NotApplicable => read(r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"ruleId":"r","kind":"notApplicable","message":{"text":"not run"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#)
            .map(|log| log.results.is_empty() && matches!(log.inventory, Inventory::Undeclared)),
        Question::EmptyRules => read(r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[]}},"results":[]}]}"#)
            .map(|log| matches!(log.inventory, Inventory::Undeclared)),
        Question::Tastemaker => read(TASTEMAKER).map(|log| {
            log.results.len() == 4
                && log
                    .results
                    .iter()
                    .all(|result| result.path.0.starts_with("/repo/crates/ryo/"))
                && log
                    .results
                    .iter()
                    .all(|result| matches!(result.supplied, Fingerprinted::Known(_)))
                && log
                    .results
                    .iter()
                    .filter(|result| matches!(result.suppression, Suppression::Suppressed(_)))
                    .count()
                    == 1
                && log
                    .results
                    .iter()
                    .filter(|result| result.place == Place::File)
                    .count()
                    == 1
                && matches!(log.inventory, Inventory::Declared(rules) if rules.len() >= 4)
        }),
        Question::Clippy => read(CLIPPY).map(|log| {
            log.results.len() == 4
                && log
                    .results
                    .iter()
                    .all(|result| &*result.path.0 == "src/lib.rs")
                && log
                    .results
                    .iter()
                    .all(|result| result.level == Level::Warning)
                && log.results.iter().all(|result| {
                    result.fingerprint == Fingerprinted::Unknown
                        && result.supplied == Fingerprinted::Unknown
                })
                && matches!(log.inventory, Inventory::Declared(rules) if rules.len() == 3)
        }),
        Question::NoLog => read("{\"runs\": 1}").map(|_| true),
        Question::EndsDiffer => read(&log(&[
            r#"{"startLine": 2, "startColumn": 9, "endLine": 2, "endColumn": 15}"#,
            r#"{"startLine": 2, "startColumn": 9, "endLine": 2, "endColumn": 16}"#,
        ]))
        .map(|log| log.results.len() == 2),
        Question::Same => read(&log(&[
            r#"{"startLine": 2, "startColumn": 9, "endLine": 2, "endColumn": 15}"#,
            r#"{"startLine": 2, "startColumn": 9, "endLine": 2, "endColumn": 15}"#,
        ]))
        .map(|log| log.results.len() == 1),
        Question::NegativeLine => read(&log(&[r#"{"startLine": -1}"#])).map(|_| true),
        Question::ZeroLine => read(&log(&[r#"{"startLine": 0}"#])).map(|_| true),
        Question::Locationless => read(r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"probe","rules":[{"id":"GLOBAL"}]}},"results":[{"ruleId":"GLOBAL","level":"error","message":{"text":"the program violates the rule"}}]}]}"#).map(|_| true),
        Question::LogicalOnly => read(r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"probe","rules":[{"id":"GLOBAL"}]}},"results":[{"ruleId":"GLOBAL","level":"error","message":{"text":"the program violates the rule"},"locations":[{"logicalLocations":[{"fullyQualifiedName":"app::main"}]}]}]}]}"#).map(|_| true),
        Question::NoRule => read(r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"probe"}},"results":[{"level":"error","message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#).map(|_| true),
        Question::OtherVersion => read(r#"{"version":"NOT SARIF","runs":[]}"#).map(|_| true),
        Question::Remote => read(&CLIPPY.replacen(
            "\"uri\": \"src/lib.rs\"",
            "\"uri\": \"https://example.com/lib.rs\"",
            1,
        ))
        .map(|_| true),
    };
    if let Err(error) = &answer {
        assert!(matches!(
            error,
            SarifError::Unparsable { .. }
                | SarifError::Unlocatable { .. }
                | SarifError::Incomplete { .. }
        ));
    }
    answer
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
