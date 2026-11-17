//! The one document of a bench run: ryo's costs under `<language>/<project>`, the verification times under
//! `<language>/verify/<rule>` and the growth under `<language>/growth/<shape>`, and nothing of the reference.
use super::{costs, encode, growth, select, verification};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::adapters::bmf::types::MeasureDocument;
use crate::outbound::evidence::fixtures::sample;
use crate::outbound::evidence::types::{MeasureClass, PublishedDocument};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The costs of django, as their names and values.
    Costs,
    /// The verification times of F401.
    Verification,
    /// The growth of one shape.
    Growth,
    /// The same project added twice.
    DuplicateProject,
    /// A document nothing was added to.
    Empty,
    /// Costs, verification times and growth written, then the deterministic measures selected.
    SelectDeterministic,
    /// The same document, its timing measures selected.
    SelectTiming,
    /// A document naming a measure of no class, a selection asked of it.
    SelectUnclassed,
    /// A document of times alone, its deterministic measures selected.
    SelectNothing,
}
type Input = Scenario;
type Value = serde_json::Value;
type Failure = Refusal;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Duplicate,
    Empty,
    Unclassed,
    NothingSelected,
    Unexpected,
}

/// A measure of the document as `[value, lower, upper]`.
fn observed(value: f64, lower: f64, upper: f64) -> serde_json::Value {
    serde_json::json!({ "value": value, "lower_value": lower, "upper_value": upper })
}

/// Milliseconds of a duration, as the document writes them.
fn millis(milliseconds: u64) -> f64 {
    Duration::from_millis(milliseconds).as_secs_f64() * 1000.0
}

/// Seconds of a duration of that many milliseconds, as the document writes them.
fn secs(milliseconds: u64) -> f64 {
    Duration::from_millis(milliseconds).as_secs_f64()
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "only ryo's cost of django is a measure, under python/django, each condition under its own name",
        ),
        word: Word::Succeeds,
        input: || Scenario::Costs,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("verification times are the bench's measures, under python/verify/<rule>"),
        word: Word::Succeeds,
        input: || Scenario::Verification,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("a shape's growth is the bench's measure, under python/growth/<shape>"),
        word: Word::Succeeds,
        input: || Scenario::Growth,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("a project added twice is refused"),
        word: Word::Refuses,
        input: || Scenario::DuplicateProject,
        expect: Expect::Error(Refusal::Duplicate),
    },
    Case {
        name: CaseName("a document nothing was added to is refused"),
        word: Word::Refuses,
        input: || Scenario::Empty,
        expect: Expect::Error(Refusal::Empty),
    },
    Case {
        name: CaseName(
            "a deterministic selection keeps the store and the peak memory, and drops every time and the benchmarks left empty",
        ),
        word: Word::Succeeds,
        input: || Scenario::SelectDeterministic,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName(
            "a timing selection keeps the cost's times, the verification times and the growth, and drops the store and memory",
        ),
        word: Word::Succeeds,
        input: || Scenario::SelectTiming,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("a measure of no class is refused, not dropped"),
        word: Word::Refuses,
        input: || Scenario::SelectUnclassed,
        expect: Expect::Error(Refusal::Unclassed),
    },
    Case {
        name: CaseName("a selection that keeps no measure is refused"),
        word: Word::Refuses,
        input: || Scenario::SelectNothing,
        expect: Expect::Error(Refusal::NothingSelected),
    },
];

/// The expected document of each succeeding scenario, compared in full by the subject.
fn expected(scenario: Scenario) -> serde_json::Value {
    match scenario {
        Scenario::Costs => serde_json::json!({
            "python/django": {
                "cpu-s": observed(secs(1_001), secs(1_000), secs(1_002)),
                "wall-s": observed(secs(1_501), secs(1_500), secs(1_502)),
                "peak-rss-bytes": observed(1_000_001.0, 1_000_000.0, 1_000_002.0),
                "warm-cpu-s": observed(secs(21), secs(20), secs(22)),
                "warm-peak-rss-bytes": observed(200_001.0, 200_000.0, 200_002.0),
                "edited-cpu-s": observed(secs(61), secs(60), secs(62)),
                "edited-peak-rss-bytes": observed(300_001.0, 300_000.0, 300_002.0),
                "store-bytes": observed(30.0, 30.0, 30.0),
                "store-files": observed(3.0, 3.0, 3.0),
            }
        }),
        Scenario::Verification => serde_json::json!({
            "python/verify/F401": {
                "total-ms": observed(millis(40), millis(40), millis(40)),
                "max-case-ms": observed(millis(9), millis(9), millis(9)),
                "compile-ms": observed(millis(3), millis(3), millis(3)),
            }
        }),
        Scenario::Growth => serde_json::json!({
            "python/growth/F401/module/assign": {
                "doubling-ratio": observed(1.9, 1.9, 1.9),
                "larger-ms": observed(millis(120), millis(120), millis(120)),
            }
        }),
        Scenario::SelectDeterministic => serde_json::json!({
            "python/django": {
                "peak-rss-bytes": observed(1_000_001.0, 1_000_000.0, 1_000_002.0),
                "warm-peak-rss-bytes": observed(200_001.0, 200_000.0, 200_002.0),
                "edited-peak-rss-bytes": observed(300_001.0, 300_000.0, 300_002.0),
                "store-bytes": observed(30.0, 30.0, 30.0),
                "store-files": observed(3.0, 3.0, 3.0),
            }
        }),
        Scenario::SelectTiming => serde_json::json!({
            "python/django": {
                "cpu-s": observed(secs(1_001), secs(1_000), secs(1_002)),
                "wall-s": observed(secs(1_501), secs(1_500), secs(1_502)),
                "warm-cpu-s": observed(secs(21), secs(20), secs(22)),
                "edited-cpu-s": observed(secs(61), secs(60), secs(62)),
            },
            "python/verify/F401": {
                "total-ms": observed(millis(40), millis(40), millis(40)),
                "max-case-ms": observed(millis(9), millis(9), millis(9)),
                "compile-ms": observed(millis(3), millis(3), millis(3)),
            },
            "python/growth/F401/module/assign": {
                "doubling-ratio": observed(1.9, 1.9, 1.9),
                "larger-ms": observed(millis(120), millis(120), millis(120)),
            }
        }),
        Scenario::DuplicateProject
        | Scenario::Empty
        | Scenario::SelectUnclassed
        | Scenario::SelectNothing => serde_json::Value::Null,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let language = "python".into();
    let mut document = MeasureDocument::default();
    let added = match scenario {
        Scenario::Costs => costs(&mut document, &language, &sample::costs()),
        Scenario::Verification => verification(&mut document, &language, &sample::timings()),
        Scenario::Growth => growth(&mut document, &language, &sample::growth()),
        Scenario::DuplicateProject => costs(&mut document, &language, &sample::costs())
            .and_then(|()| costs(&mut document, &language, &sample::costs())),
        Scenario::Empty => Ok(()),
        Scenario::SelectDeterministic | Scenario::SelectTiming => {
            costs(&mut document, &language, &sample::costs())
                .and_then(|()| verification(&mut document, &language, &sample::timings()))
                .and_then(|()| growth(&mut document, &language, &sample::growth()))
        }
        Scenario::SelectUnclassed => {
            return selected(
                b"{\"python/django\":{\"heap-mb\":{\"value\":1.0}}}",
                MeasureClass::Timing,
            );
        }
        Scenario::SelectNothing => {
            return selected(
                b"{\"python/django\":{\"cpu-s\":{\"value\":1.0}}}",
                MeasureClass::Deterministic,
            );
        }
    };
    added.map_err(|error| {
        if error.to_string().contains("twice") {
            Refusal::Duplicate
        } else {
            Refusal::Unexpected
        }
    })?;
    let encoded = encode(&document).map_err(|_empty| Refusal::Empty)?;
    let bytes = match scenario {
        Scenario::SelectDeterministic => {
            select(&PublishedDocument(encoded), MeasureClass::Deterministic)
                .map_err(|_refused| Refusal::Unexpected)?
                .0
        }
        Scenario::SelectTiming => {
            select(&PublishedDocument(encoded), MeasureClass::Timing)
                .map_err(|_refused| Refusal::Unexpected)?
                .0
        }
        Scenario::Costs
        | Scenario::Verification
        | Scenario::Growth
        | Scenario::DuplicateProject
        | Scenario::Empty
        | Scenario::SelectUnclassed
        | Scenario::SelectNothing => encoded,
    };
    let written: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_malformed| Refusal::Unexpected)?;
    // The whole document is compared: a reference benchmark, a missing measure or a value under another condition's name
    // makes it differ.
    if written == expected(scenario) {
        Ok(serde_json::Value::Null)
    } else {
        Ok(written)
    }
}

/// The refusal of selecting `class` of `document`, whose answer is never a document.
fn selected(document: &[u8], class: MeasureClass) -> Result<Value, Failure> {
    match select(&PublishedDocument(document.to_vec()), class) {
        Ok(kept) => Ok(serde_json::from_slice(&kept.0).map_err(|_malformed| Refusal::Unexpected)?),
        Err(refused)
            if refused
                .to_string()
                .contains("neither deterministic nor timing") =>
        {
            Err(Refusal::Unclassed)
        }
        Err(refused)
            if refused
                .to_string()
                .contains("no measure of the class selected") =>
        {
            Err(Refusal::NothingSelected)
        }
        Err(_refused) => Err(Refusal::Unexpected),
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
