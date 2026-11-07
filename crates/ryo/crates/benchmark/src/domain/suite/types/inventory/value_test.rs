//! Proves one value's behaviour: an inventory starts failed and passes only by evidence.
use super::value::{FailureReason, Inventory, PassedRule, InventoryVerdict};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::requests::types::RuleLabel;
use ryo_benchmark_contracts::domain::workloads::types::case_count::value::CaseCount;

/// What a scenario does to a two-rule inventory.
#[derive(Clone, Copy)]
enum Scenario {
    Untouched,
    OnePasses,
    BothPass,
    PassThenFail,
    Duplicate,
    Empty,
    Unsafe,
}

type Input = Scenario;
type Value = InventoryVerdict;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a fresh inventory has not passed"),
        word: Word::Succeeds,
        input: || Scenario::Untouched,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("one passing rule leaves the inventory failed"),
        word: Word::Succeeds,
        input: || Scenario::OnePasses,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("every rule passing passes the inventory"),
        word: Word::Succeeds,
        input: || Scenario::BothPass,
        expect: Expect::Value(InventoryVerdict::Passed),
    },
    Case {
        name: CaseName("failing every rule after a pass returns the inventory to failed"),
        word: Word::Succeeds,
        input: || Scenario::PassThenFail,
        expect: Expect::Value(InventoryVerdict::Failed),
    },
    Case {
        name: CaseName("a duplicate rule is refused"),
        word: Word::Refuses,
        input: || Scenario::Duplicate,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("an empty inventory is refused"),
        word: Word::Refuses,
        input: || Scenario::Empty,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("an escaping rule identity is refused"),
        word: Word::Refuses,
        input: || Scenario::Unsafe,
        expect: Expect::Error(()),
    },
];

fn label(text: &str) -> RuleLabel {
    RuleLabel(text.into())
}

fn evidence() -> PassedRule {
    PassedRule {
        cases: CaseCount(1),
        raw_exact: CaseCount(1),
        documented: CaseCount(0),
        contract_only: CaseCount(0),
        case_results_sha256: ArtifactDigest("digest".into()),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let rules = match scenario {
        Scenario::Duplicate => vec![label("A"), label("A")],
        Scenario::Empty => Vec::new(),
        Scenario::Unsafe => vec![label("../A")],
        _ => vec![label("A"), label("B")],
    };
    let mut inventory = Inventory::new(&rules).map_err(|_| ())?;
    match scenario {
        Scenario::OnePasses => inventory.pass(&label("A"), evidence()),
        Scenario::BothPass => {
            inventory.pass(&label("A"), evidence());
            inventory.pass(&label("B"), evidence());
        }
        Scenario::PassThenFail => {
            inventory.pass(&label("A"), evidence());
            inventory.fail_all(&FailureReason("again".into()));
        }
        _ => {}
    }
    Ok(inventory.verdict())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
