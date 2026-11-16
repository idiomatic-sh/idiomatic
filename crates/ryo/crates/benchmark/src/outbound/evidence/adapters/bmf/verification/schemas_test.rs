//! The verification times of a rule are written as the three BMF measures of one benchmark named for linter and rule.
use super::encode;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::types::RuleTiming;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use std::collections::BTreeMap;
use std::time::Duration;

/// The seconds the rule's cases took together, at the slowest, and its compile.
type Input = (u64, u64, u64);
/// The total, slowest-case and compile measures the document holds for the rule, in milliseconds.
type Value = (f64, f64, f64);
/// The refusal's message.
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a rule's times are its three measures, in milliseconds"),
    word: Word::Succeeds,
    input: || (3, 2, 1),
    expect: Expect::Value((3000.0, 2000.0, 1000.0)),
}];

/// Writes the times of the rule `R1` of the linter `lint` and reads its measures back.
fn subject((total, slowest, compile): Input) -> Result<Value, Failure> {
    let timings = BTreeMap::from([(
        ryo::linter::RuleId::try_from("R1").map_err(|error| error.to_string())?,
        RuleTiming {
            total: Duration::from_secs(total),
            slowest: Duration::from_secs(slowest),
            compile: Duration::from_secs(compile),
        },
    )]);
    let bytes = encode(&ToolName("lint".into()), &timings).map_err(|error| error.to_string())?;
    let document: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let measure = |name: &str| {
        document["verify/lint/R1"][name]["value"]
            .as_f64()
            .ok_or_else(|| format!("no {name} measure"))
    };
    Ok((
        measure("total-ms")?,
        measure("max-case-ms")?,
        measure("compile-ms")?,
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
