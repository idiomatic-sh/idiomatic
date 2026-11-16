//! Every verified rule reports the time of its own cases, and a rule the profile only loads for another's sake reports none.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::verify_cases::fixtures::shard::{Shape, prepared, succeeding};
use crate::inbound::use_cases::verify_cases::service::verify_prepared_shard;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::{
    execution::fixtures::fake::ExecutionFake, linter::fixtures::fake::LinterFake,
    receipts::fixtures::fake::ReceiptsFake,
};
use ryo_benchmark_contracts::domain::measurements::types::{
    ExecutionProfile, Invocation, ProcessOutput,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::CaseSelection;
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

/// Which rules the execution reports compiling ahead of the cases.
#[derive(Clone, Copy)]
enum Compiled {
    /// Only the rule the case belongs to.
    CaseRule,
    /// That rule and a supporting rule the profile loads, which has no case.
    WithSupportingRule,
}
type Input = Compiled;
/// How many rules report times, and whether each reports a nonzero total.
type Value = (usize, bool);
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the rule of the case reports a nonzero total"),
        word: Word::Succeeds,
        input: || Compiled::CaseRule,
        expect: Expect::Value((1, true)),
    },
    Case {
        name: CaseName("a supporting rule compiled beside it reports no time of its own"),
        word: Word::Succeeds,
        input: || Compiled::WithSupportingRule,
        expect: Expect::Value((1, true)),
    },
];

/// The fake execution, which also reports compiling the rules it is given.
struct CompilingExecution {
    inner: ExecutionFake,
    compiled: Vec<ryo::linter::RuleId>,
}
impl ExecutionPort for CompilingExecution {
    fn profile(&self) -> ExecutionProfile {
        self.inner.profile()
    }
    fn compile(&self, _invocation: &Invocation) -> Option<BTreeMap<ryo::linter::RuleId, Duration>> {
        Some(
            self.compiled
                .iter()
                .map(|rule| (rule.clone(), Duration::from_millis(48)))
                .collect(),
        )
    }
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, ReportsError> {
        self.inner.run(invocation)
    }
}

fn subject(compiled: Input) -> Result<Value, Failure> {
    let (suite, expected) = prepared(Shape::Agreement);
    let reference = LinterFake {
        diagnostics: vec![expected.clone()],
        ..Default::default()
    };
    let candidate = LinterFake {
        diagnostics: vec![expected],
        ..Default::default()
    };
    let mut rules = suite
        .prepared
        .iter()
        .map(|case| case.rule.clone())
        .collect::<Vec<_>>();
    if matches!(compiled, Compiled::WithSupportingRule) {
        rules.push(ryo::linter::RuleId::try_from("SUPPORT").map_err(|error| error.to_string())?);
    }
    let execution = CompilingExecution {
        inner: succeeding(),
        compiled: rules,
    };
    let verified = verify_prepared_shard(
        &suite,
        &crate::outbound::host::fixtures::fake::HostFake::default(),
        &ReceiptsFake::default(),
        &crate::outbound::context::factory::local(),
        &reference,
        &candidate,
        &execution,
        &crate::outbound::inventory::factory::local(),
        Path::new("rules"),
        suite.prepared.clone(),
        CaseSelection::default(),
    )
    .map_err(|error| error.to_string())?;
    Ok((
        verified.timings.len(),
        verified
            .timings
            .values()
            .all(|timing| timing.total > Duration::ZERO && timing.compile > Duration::ZERO),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
