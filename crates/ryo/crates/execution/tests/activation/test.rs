//! Actual derived activation visits and allocator blocks exclude dormant compiled roots.
mod fixtures;
use fixtures::file::measure;
use fixtures::project::measure_project;
use fixtures::subscriber::{Access, Sample};
use ryo_execution::inbound::use_cases::execution::types::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

#[derive(Clone, Copy)]
enum Input {
    Empty,
    Sparse,
    Selector,
    Graph,
    Mixed,
    TailSelectors,
    Dense,
    ProjectSparse,
    ProjectDense,
    ProjectUncounted,
}
fn same_cost(left: Sample, right: Sample) {
    assert_eq!(left.work, right.work);
    assert_eq!(left.graph_hits, right.graph_hits);
    assert_eq!(
        left.blocks, right.blocks,
        "dormant roots cannot add activation allocations"
    );
    assert_eq!(
        left.bytes, right.bytes,
        "dormant roots cannot grow activation allocations"
    );
}
/// Equal counted accesses, work and allocations: nothing but the demanded roots was touched.
fn project_cost_matches(left: (Sample, Access), right: (Sample, Access)) -> bool {
    left.0 == right.0 && left.1.reads == right.1.reads
}
fn subject(input: Input) -> Result<(), OperatingError> {
    const TOTAL: usize = 131;
    match input {
        Input::Sparse => {
            let sparse = measure(TOTAL, &[0]);
            assert_eq!(
                sparse.work, [1; 8],
                "sparse intent must not read dormant rule metadata"
            );
        }
        Input::Empty => {
            let small = measure(3, &[]);
            let dormant = measure(TOTAL, &[]);
            assert_eq!(small, Sample::default());
            assert_eq!(dormant, Sample::default());
        }
        Input::Selector => {
            let small = measure(3, &[0]);
            let dormant = measure(TOTAL, &[0]);
            assert_eq!(small.work, [1; 8]);
            assert_eq!(small.graph_hits, 0);
            assert!(small.blocks > 0 && small.bytes > 0);
            assert_eq!(
                small, dormant,
                "sparse intent must not allocate dormant metadata"
            );
        }
        Input::Graph => {
            let small = measure(3, &[1]);
            let tail = measure(TOTAL, &[129]);
            assert_eq!(small.work, [1, 3, 1, 1, 0, 0, 0, 0]);
            assert_eq!(small.graph_hits, 1);
            assert_eq!(small.rule_sums, [1; 3]);
            assert_eq!(tail.rule_sums, [129; 3]);
            same_cost(small, tail);
        }
        Input::Mixed => {
            let small = measure(3, &[0, 1, 2]);
            let dormant = measure(TOTAL, &[0, 1, 2]);
            assert_eq!(small.work, [3, 5, 2, 3, 2, 2, 2, 2]);
            assert_eq!(small.rule_sums, [3, 1, 3]);
            assert_eq!(small.graph_hits, 1);
            assert_eq!(
                small, dormant,
                "sparse intent must not allocate dormant metadata"
            );
        }
        Input::TailSelectors => {
            let small = measure(3, &[0, 2]);
            let tail = measure(TOTAL, &[130, 0]);
            assert_eq!(small.work, [2; 8]);
            assert_eq!(small.rule_sums, [2; 3]);
            assert_eq!(tail.rule_sums, [130; 3]);
            same_cost(small, tail);
        }
        Input::Dense => {
            let sparse = measure(TOTAL, &[0]);
            let dense = measure(TOTAL, &(0..TOTAL).collect::<Vec<_>>());
            assert_eq!(dense.work, [131, 261, 2, 131, 66, 66, 66, 66]);
            assert!(dense.blocks > sparse.blocks && dense.bytes > sparse.bytes);
            // The negative control offers the actual all-demand activation to
            // the sparse oracle. It must reject extra work and allocations.
            // This is the same production entry. This is demand sensitivity, not an implementation mutant.
            let sparse_holds = |sample: Sample| {
                sample.work == sparse.work
                    && sample.blocks <= sparse.blocks
                    && sample.bytes <= sparse.bytes
            };
            assert!(sparse_holds(sparse));
            assert!(
                !sparse_holds(dense),
                "the oracle must falsify dense processing"
            );
        }
        Input::ProjectSparse => {
            let small = measure_project(3, &[1], &|_| {});
            let tail = measure_project(131, &[129], &|_| {});
            assert!(
                small.1.reads > 0 && tail.1.reads > 0,
                "the demanded project root reads its own metadata"
            );
            assert_eq!(small.1.read_rules, small.1.reads);
            assert_eq!(
                tail.1.read_rules,
                129 * tail.1.reads,
                "every counted access names the demanded rule, never a dormant one"
            );
            assert_eq!(small.1.reads, tail.1.reads);
            same_cost(small.0, tail.0);
        }
        Input::ProjectDense => {
            let sparse = measure_project(131, &[0], &|_| {});
            let dense = measure_project(131, &(0..131).collect::<Vec<_>>(), &|_| {});
            assert!(dense.1.reads > sparse.1.reads);
            assert!(dense.0.blocks > sparse.0.blocks && dense.0.bytes > sparse.0.bytes);
            let sparse_holds = |sample: (Sample, Access)| {
                sample.1.reads == sparse.1.reads
                    && sample.0.blocks <= sparse.0.blocks
                    && sample.0.bytes <= sparse.0.bytes
            };
            assert!(sparse_holds(sparse));
            assert!(
                !sparse_holds(dense),
                "the oracle must falsify dense project processing"
            );
        }
        Input::ProjectUncounted => {
            // A scan over every dormant rule that reports no counter. Only the
            // independent allocation measurement can see it.
            let scan = |total: usize| {
                std::hint::black_box((0..total).map(|rule| vec![rule]).collect::<Vec<_>>());
            };
            let small = measure_project(3, &[1], &|_| {});
            let honest = measure_project(131, &[129], &|_| {});
            let uncounted = measure_project(131, &[129], &scan);
            assert_eq!(
                honest.1, uncounted.1,
                "the uncounted scan leaves every counter unchanged"
            );
            assert!(project_cost_matches(small, honest));
            assert!(
                !project_cost_matches(small, uncounted),
                "allocation measurement must detect an uncounted dormant scan"
            );
        }
    }
    Ok(())
}
const CASES: &[Case<Input, (), OperatingError>] = &[
    Case {
        name: CaseName("same_131_rule_sparse_intent_reads_only_its_required_metadata"),
        word: Word::Succeeds,
        input: || Input::Sparse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "rootless_activation_has_zero_work_and_allocations_with_dense_dormant_metadata",
        ),
        word: Word::Succeeds,
        input: || Input::Empty,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "one_selector_has_identical_actual_work_and_allocations_with_128_dormant_roots",
        ),
        word: Word::Succeeds,
        input: || Input::Selector,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("tail_graph_activation_probes_only_the_relevant_rule"),
        word: Word::Succeeds,
        input: || Input::Graph,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("mixed_graph_and_selector_activation_scales_with_three_relevant_roots"),
        word: Word::Succeeds,
        input: || Input::Mixed,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("noncontiguous_selector_intents_use_their_own_sorted_rule_ranges"),
        word: Word::Succeeds,
        input: || Input::TailSelectors,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "actual_dense_activation_is_rejected_by_the_sparse_work_and_allocation_oracle",
        ),
        word: Word::Succeeds,
        input: || Input::Dense,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "project_roots_cost_the_same_actual_work_and_allocations_with_128_dormant_roots",
        ),
        word: Word::Succeeds,
        input: || Input::ProjectSparse,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "actual_dense_project_submission_is_rejected_by_the_sparse_work_and_allocation_oracle",
        ),
        word: Word::Succeeds,
        input: || Input::ProjectDense,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_uncounted_dormant_project_scan_is_detected_by_allocation"),
        word: Word::Succeeds,
        input: || Input::ProjectUncounted,
        expect: Expect::Value(()),
    },
];
fn main() {
    // This target is alone in its process; assertions count blocks and bytes,
    // never elapsed time. Request compilation and full intent binding precede
    // the observed derived activation boundary and are outside this claim; the
    // project cases observe phase opening through the last root delivery.
    let _profiler = dhat::Profiler::builder().testing().build();
    let selected = std::env::args().nth(1);
    let mut executed = 0;
    for case in CASES
        .iter()
        .filter(|case| selected.as_deref().is_none_or(|name| name == case.name.0))
    {
        run(std::slice::from_ref(case), Word::Succeeds, subject);
        executed += 1;
    }
    assert!(
        executed > 0,
        "the activation target must execute an actual case"
    );
    println!("activation: {executed} actual cases passed");
}
