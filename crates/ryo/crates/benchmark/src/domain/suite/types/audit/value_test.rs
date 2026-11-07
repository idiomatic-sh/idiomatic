//! Proves one value's behaviour.
use super::value::Audit;
use crate::cases::{Case, CaseName, Expect, Word, run};
const CASES: &[Case<Vec<ryo::linter::RuleId>, usize, ()>] = &[
    Case {
        name: CaseName("authored subset is retained"),
        word: Word::Succeeds,
        input: || vec![ryo::linter::RuleId::try_from("A").unwrap()],
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("missing rule cannot be selected"),
        word: Word::Refuses,
        input: || vec![ryo::linter::RuleId::try_from("B").unwrap()],
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("empty selection cannot certify"),
        word: Word::Refuses,
        input: Vec::new,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("duplicate selection cannot certify"),
        word: Word::Refuses,
        input: || {
            vec![
                ryo::linter::RuleId::try_from("A").unwrap(),
                ryo::linter::RuleId::try_from("A").unwrap(),
            ]
        },
        expect: Expect::Error(()),
    },
];
#[expect(
    clippy::needless_pass_by_value,
    reason = "the shared case runner hands each independently constructed input to the subject by value"
)]
fn subject(rules: Vec<ryo::linter::RuleId>) -> Result<usize, ()> {
    let audit = Audit {
        implemented: vec![ryo::linter::RuleId::try_from("A").unwrap()],
        missing: vec![ryo::linter::RuleId::try_from("B").unwrap()],
    };
    assert_eq!(audit.authored_count().0, 1);
    audit
        .select(&rules)
        .map(|selected| selected.authored().len())
        .map_err(|_error| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn exact_selected_authored_and_missing_inventory_preserves_sorted_unselected_defaults() {
    let audit = Audit {
        implemented: vec!["E722".try_into().unwrap(), "F704".try_into().unwrap()],
        missing: vec!["F401".try_into().unwrap()],
    };
    let selected = audit.select(&["E722".try_into().unwrap()]).unwrap();
    assert_eq!(
        selected
            .implemented
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["E722"]
    );
    assert_eq!(
        selected
            .missing
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["F401", "F704"]
    );
}
