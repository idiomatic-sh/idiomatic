//! The nested rule documents accept their exact shapes and refuse unknown or missing keys.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::worker::adapters::json::types::DocumentWire;
use ryo_benchmark_contracts::domain::context::types::PreparedRuleContext;
use ryo_benchmark_contracts::domain::context::types::rule_observation_declarations::value::RuleObservationDeclarations;

/// Which nested document a row decodes.
#[derive(Clone, Copy, Debug)]
enum Document {
    Context,
    Observations,
}
type Input = (Document, serde_json::Value);
type Value = ();
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a complete rule context is accepted"),
        word: Word::Succeeds,
        input: || {
            (
                Document::Context,
                serde_json::json!({
                    "tables": {"selected": "a", "disabled": "b", "external": "c"},
                    "configuration": {"external-prefixes": ["V1"], "disabled-rules": {}},
                }),
            )
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a rule context with an undeclared key is refused"),
        word: Word::Refuses,
        input: || {
            (
                Document::Context,
                serde_json::json!({
                    "tables": {"selected": "a", "disabled": "b", "external": "c"},
                    "configuration": {"external-prefixes": [], "disabled-rules": {}},
                    "extra": true,
                }),
            )
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("an empty declaration map is accepted"),
        word: Word::Succeeds,
        input: || (Document::Observations, serde_json::json!({})),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a binding with an unknown producer kind is refused"),
        word: Word::Refuses,
        input: || {
            (
                Document::Observations,
                serde_json::json!({"A001": [{"name": "n", "columns": [], "producers": [{"kind": "other"}]}]}),
            )
        },
        expect: Expect::Error(()),
    },
];
fn subject((document, value): Input) -> Result<Value, Failure> {
    match document {
        Document::Context => PreparedRuleContext::try_from(DocumentWire(value))
            .map(|_| ())
            .map_err(|_refused| ()),
        Document::Observations => RuleObservationDeclarations::try_from(DocumentWire(value))
            .map(|_| ())
            .map_err(|_refused| ()),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
