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
    Case {
        name: CaseName("independent_allocator_rejects_request_sized_repetition_reservation"),
        word: Word::Rejects,
        input: || Mutation::DenseCapacity,
        expect: Expect::Error(Rejected::DormantAllocation),
    },
];
#[test]
fn rejects() {
    let actual = ActualSource::new();
    actual.baseline();
    run(CASES, Word::Rejects, |mutation| actual.reject(mutation));
}
