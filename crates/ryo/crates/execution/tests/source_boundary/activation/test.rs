//! Same-input source mutants must fail the real public activation controls.
use super::fixtures::{ActualSource, Mutation, Rejected};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
const CASES: &[Case<Mutation, (), Rejected>] = &[
    Case {
        name: CaseName("required_lookup_rejects_dormant_reads_before_the_original_active_lookup"),
        word: Word::Rejects,
        input: || Mutation::DenseReads,
        expect: Expect::Error(Rejected::DormantReads),
    },
];
#[test]
fn rejects() {
    let actual = ActualSource::new();
    actual.baseline();
    run(CASES, Word::Rejects, |mutation| actual.reject(mutation));
}
