//! Declared identity equality, never just equal rule counts, gates benchmarks.
use super::service::require_inventory;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::measurements::types::MeasurementMode;
use ryo::linter::RuleId;
type Input = (MeasurementMode, Vec<RuleId>, Option<Vec<RuleId>>);
fn ids(names: &[&str]) -> Vec<RuleId> {
    names
        .iter()
        .map(|name| RuleId::try_from(*name).unwrap())
        .collect()
}
fn benchmark(authored: &[&str], selected: Option<&[&str]>) -> Input {
    (
        MeasurementMode::Benchmark {
            inventory: ids(&["A", "B"]),
        },
        ids(authored),
        selected.map(ids),
    )
}
const CASES: &[Case<Input, (), ()>] = &[
    Case {
        name: CaseName("full inventory accepts reordered identities"),
        word: Word::Succeeds,
        input: || benchmark(&["B", "A"], Some(&["A", "B"])),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("narrow diagnostic is explicitly allowed"),
        word: Word::Succeeds,
        input: || (MeasurementMode::Diagnostic, ids(&["A"]), Some(ids(&["A"]))),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("narrowed authored default rewrite refused"),
        word: Word::Refuses,
        input: || benchmark(&["A"], Some(&["A"])),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("narrow invocation refused despite complete certificate"),
        word: Word::Refuses,
        input: || benchmark(&["A", "B"], Some(&["A"])),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("same count wrong identity refused"),
        word: Word::Refuses,
        input: || benchmark(&["A", "B"], Some(&["A", "C"])),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("duplicate selected identity refused"),
        word: Word::Refuses,
        input: || benchmark(&["A", "B"], Some(&["A", "B", "B"])),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("unresolved benchmark selection refused"),
        word: Word::Refuses,
        input: || benchmark(&["A", "B"], None),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("duplicate declared inventory refused"),
        word: Word::Refuses,
        input: || {
            (
                MeasurementMode::Benchmark {
                    inventory: ids(&["A", "A"]),
                },
                ids(&["A"]),
                Some(ids(&["A"])),
            )
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("extra authored identity refused"),
        word: Word::Refuses,
        input: || benchmark(&["A", "B", "C"], Some(&["A", "B"])),
        expect: Expect::Error(()),
    },
];
fn subject((mode, authored, selected): Input) -> Result<(), ()> {
    require_inventory(&mode, &authored, selected.as_deref()).map_err(|_| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
