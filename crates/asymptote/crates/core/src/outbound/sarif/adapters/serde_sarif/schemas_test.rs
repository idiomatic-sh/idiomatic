//! What the log schemas resolve a result's location and its rule to.

use super::{LogSchema, Read};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row reads.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// A result's artifact location and the run's bases, as JSON, resolved to the result's path.
    Located(&'static str, &'static str),
    /// A whole log, read to its results' rule ids, comma-joined: how results name their rules by id, index and tool
    /// component, and the invocations and kinds that hold no violation.
    Ruled(&'static str),
}

/// Which question is asked.
type Input = Question;
/// The path a location resolves to, or the rule ids a log's results keep.
type Value = std::borrow::Cow<'static, str>;
/// The refusal's text.
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_relative_uri_is_the_path_as_written"),
        word: Word::Succeeds,
        input: || Question::Located(r#"{"uri": "src/a%20b.rs"}"#, "{}"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("src/a b.rs")),
    },
    Case {
        name: CaseName("a_base_id_resolves_through_the_runs_bases"),
        word: Word::Succeeds,
        input: || {
            Question::Located(
                r#"{"uri": "src/a.rs", "uriBaseId": "ROOT"}"#,
                r#"{"ROOT": {"uri": "file:///repo/"}}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("/repo/src/a.rs")),
    },
    Case {
        name: CaseName("a_base_id_the_run_does_not_hold_is_refused"),
        word: Word::Refuses,
        input: || Question::Located(r#"{"uri": "src/a.rs", "uriBaseId": "NONE"}"#, "{}"),
        expect: Expect::Refusal(|actual| {
            if actual
                == "a SARIF result of `r` cannot be located: uriBaseId \"NONE\" is not in originalUriBaseIds"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("malformed_percent_encoding_is_refused"),
        word: Word::Refuses,
        input: || Question::Located(r#"{"uri": "src/a%zz.rs"}"#, "{}"),
        expect: Expect::Refusal(|actual| {
            if actual
                == "a SARIF result of `r` cannot be located: the uri has malformed percent encoding"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_extension_rule_index_resolves_its_own_component"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"wrong"}]},"extensions":[{"name":"e","rules":[{"id":"r"}]}]},"results":[{"rule":{"index":0,"toolComponent":{"index":0}},"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("r")),
    },
    Case {
        name: CaseName("an_explicit_driver_component_resolves_its_rule"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"rule":{"index":0,"toolComponent":{"name":"t"}},"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("r")),
    },
    Case {
        name: CaseName("explicit_default_rule_indices_are_absent"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t"}},"results":[{"ruleId":"r","ruleIndex":-1,"rule":{"index":-1},"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("r")),
    },
    Case {
        name: CaseName("default_outer_and_component_indices_preserve_real_nested_index"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"ruleIndex":-1,"rule":{"index":0,"toolComponent":{"name":"t","index":-1}},"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("r")),
    },
    Case {
        name: CaseName("conflicting_rule_indices_are_refused"),
        word: Word::Refuses,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"ruleIndex":0,"rule":{"index":1},"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == "the output is not a SARIF log: result.ruleIndex disagrees with result.rule.index"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("an_unsuccessful_invocation_is_refused_even_with_no_results"),
        word: Word::Refuses,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t"}},"invocations":[{"executionSuccessful":false}],"results":[]}]}"#,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual == "SARIF tool `t` reported an unsuccessful invocation" {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_successful_invocation_with_no_findings_is_clean"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t"}},"invocations":[{"executionSuccessful":true}],"results":[]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_rule_reference_id_is_retained_without_rule_id"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"rule":{"id":"r","index":0},"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("r")),
    },
    Case {
        name: CaseName("a_rule_index_resolves_the_declared_rule"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"ruleIndex":0,"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("r")),
    },
    Case {
        name: CaseName("a_rule_index_outside_the_rules_is_refused"),
        word: Word::Refuses,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"ruleIndex":1,"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == "the output is not a SARIF log: result rule index is outside the component's rules"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("conflicting_rule_ids_are_refused"),
        word: Word::Refuses,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t"}},"results":[{"ruleId":"r","rule":{"id":"other"},"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Refusal(|actual| {
            if actual
                == "the output is not a SARIF log: result.ruleId disagrees with result.rule.id"
            {
                crate::cases::Selection::Matches
            } else {
                crate::cases::Selection::Differs
            }
        }),
    },
    Case {
        name: CaseName("a_hierarchical_rule_id_is_preserved_beside_its_index"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t","rules":[{"id":"r"}]}},"results":[{"ruleId":"r/detail","ruleIndex":0,"message":{"text":"m"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("r/detail")),
    },
    Case {
        name: CaseName("a_successful_rule_result_is_not_a_violation"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t"}},"results":[{"ruleId":"r","kind":"pass","level":"none","message":{"text":"ok"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("an_informational_rule_result_is_not_a_violation"),
        word: Word::Succeeds,
        input: || {
            Question::Ruled(
                r#"{"version":"2.1.0","runs":[{"tool":{"driver":{"name":"t"}},"results":[{"ruleId":"r","kind":"informational","message":{"text":"info"},"locations":[{"physicalLocation":{"artifactLocation":{"uri":"a.rs"}}}]}]}]}"#,
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
];

/// The path the result at `location`, under the run's `bases`, resolves to.
fn located(location: &str, bases: &str) -> Result<String, Failure> {
    let log = format!(
        r#"{{"version": "2.1.0", "runs": [{{"tool": {{"driver": {{"name": "t"}}}}, "originalUriBaseIds": {bases}, "results": [{{"ruleId": "r", "message": {{"text": "m"}}, "locations": [{{"physicalLocation": {{"artifactLocation": {location}}}}}]}}]}}]}}"#
    );
    let sarif = serde_json::from_str(&log).map_err(|error| error.to_string())?;
    let read = Read::try_from(LogSchema(sarif)).map_err(|error| error.to_string())?;
    read.results
        .first()
        .map(|result| result.path.0.to_string())
        .ok_or_else(|| "no result".to_owned())
}

/// The rule ids, comma-joined, the results of the log `text` keep.
fn ruled(text: &str) -> Result<String, Failure> {
    let log = serde_json::from_str(text).map_err(|error| error.to_string())?;
    super::read(log)
        .map(|read| {
            read.results
                .iter()
                .map(|result| result.rule.0.as_ref())
                .collect::<Vec<_>>()
                .join(",")
        })
        .map_err(|error| error.to_string())
}

fn subject(question: Input) -> Result<Value, Failure> {
    match question {
        Question::Located(location, bases) => located(location, bases),
        Question::Ruled(text) => ruled(text),
    }
    .map(std::borrow::Cow::Owned)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
