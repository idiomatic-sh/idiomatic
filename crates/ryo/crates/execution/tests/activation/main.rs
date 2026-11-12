//! Actual derived activation visits and allocator blocks exclude dormant compiled roots.
#[cfg(test)]
mod fixtures;
use fixtures::file::measure;
use fixtures::project::measure_project;
use fixtures::subscriber::{Access, Sample, Window, project_cost_matches, same_cost};
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use std::cell::RefCell;

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
/// What a case compared, in counters (exact) and allocation relations (allocation sizes are the allocator's).
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    /// The rule-relation lookups, relation visits and graph probes of one measurement.
    Work([u64; 3]),
    /// Two whole samples.
    Samples([Sample; 2]),
    /// The small measurement's counters, whether it allocated, and whether the large one is the same sample.
    Selector {
        work: [u64; 3],
        graph_hits: u64,
        allocates: bool,
        same: bool,
    },
    /// The small measurement's counters, both visited rule sums, whether the large one costs the same, and whether it is
    /// the same sample.
    Tail {
        work: [u64; 3],
        graph_hits: u64,
        rule_sums: [[u64; 2]; 2],
        same_cost: bool,
        same: bool,
    },
    /// The dense measurement's work, whether its allocations grew over the sparse one's, and the sparse oracle's verdict
    /// on the sparse and the dense measurement.
    Dense {
        work: [u64; 3],
        grows: bool,
        oracle: [bool; 2],
    },
    /// Both measurements' counted accesses, and whether they cost the same.
    ProjectSparse {
        accesses: [Access; 2],
        same_cost: bool,
    },
    /// The sparse and dense reads, whether the allocations grew, and the sparse oracle's verdict on each.
    ProjectDense {
        reads: [u64; 2],
        grows: bool,
        oracle: [bool; 2],
    },
    /// Whether the uncounted scan left the counters unchanged, and whether the small cost matches the honest and the
    /// uncounted measurement.
    Uncounted {
        counters_unchanged: bool,
        matches: [bool; 2],
    },
}
/// Every measurement's window, in order, and what the case compared.
type Value = (Cow<'static, [Window]>, Answer);
fn subject(input: Input) -> Result<Value, OperatingError> {
    const TOTAL: usize = 131;
    let windows = RefCell::new(vec![]);
    let measure = |total: usize, demanded: &[usize]| {
        let (sample, window) = measure(total, demanded);
        windows.borrow_mut().push(window);
        sample
    };
    let measure_project = |total: usize, demanded: &[usize], intrusion: &dyn Fn(usize)| {
        let (sample, access, window) = measure_project(total, demanded, intrusion);
        windows.borrow_mut().push(window);
        (sample, access)
    };
    let answer = match input {
        Input::Sparse => Answer::Work(measure(TOTAL, &[0]).work),
        Input::Empty => Answer::Samples([measure(3, &[]), measure(TOTAL, &[])]),
        Input::Selector => {
            let small = measure(3, &[0]);
            let dormant = measure(TOTAL, &[0]);
            Answer::Selector {
                work: small.work,
                graph_hits: small.graph_hits,
                allocates: small.blocks > 0 && small.bytes > 0,
                same: small == dormant,
            }
        }
        Input::Graph | Input::Mixed | Input::TailSelectors => {
            let (small, tail) = match input {
                Input::Graph => (measure(3, &[1]), measure(TOTAL, &[129])),
                Input::Mixed => (measure(3, &[0, 1, 2]), measure(TOTAL, &[0, 1, 2])),
                Input::TailSelectors
                | Input::Empty
                | Input::Sparse
                | Input::Selector
                | Input::Dense
                | Input::ProjectSparse
                | Input::ProjectDense
                | Input::ProjectUncounted => (measure(3, &[0, 2]), measure(TOTAL, &[130, 0])),
            };
            Answer::Tail {
                work: small.work,
                graph_hits: small.graph_hits,
                rule_sums: [small.rule_sums, tail.rule_sums],
                same_cost: same_cost(small, tail),
                same: small == tail,
            }
        }
        Input::Dense => {
            let sparse = measure(TOTAL, &[0]);
            let dense = measure(TOTAL, &(0..TOTAL).collect::<Vec<_>>());
            // The negative control offers the actual all-demand activation to the sparse oracle, which must reject its
            // extra work and allocations: demand sensitivity through the same production entry, not an implementation mutant.
            let sparse_holds = |sample: Sample| {
                sample.work == sparse.work
                    && sample.blocks <= sparse.blocks
                    && sample.bytes <= sparse.bytes
            };
            Answer::Dense {
                work: dense.work,
                // Dense activation admits more relations: its allocations grow in bytes, not necessarily in number.
                grows: dense.blocks >= sparse.blocks && dense.bytes > sparse.bytes,
                oracle: [sparse_holds(sparse), sparse_holds(dense)],
            }
        }
        Input::ProjectSparse => {
            let small = measure_project(3, &[1], &|_| {});
            let tail = measure_project(TOTAL, &[129], &|_| {});
            Answer::ProjectSparse {
                accesses: [small.1, tail.1],
                same_cost: same_cost(small.0, tail.0),
            }
        }
        Input::ProjectDense => {
            let sparse = measure_project(TOTAL, &[0], &|_| {});
            let dense = measure_project(TOTAL, &(0..TOTAL).collect::<Vec<_>>(), &|_| {});
            let sparse_holds = |sample: (Sample, Access)| {
                sample.1.reads == sparse.1.reads
                    && sample.0.blocks <= sparse.0.blocks
                    && sample.0.bytes <= sparse.0.bytes
            };
            Answer::ProjectDense {
                reads: [sparse.1.reads, dense.1.reads],
                grows: dense.0.blocks > sparse.0.blocks && dense.0.bytes > sparse.0.bytes,
                oracle: [sparse_holds(sparse), sparse_holds(dense)],
            }
        }
        Input::ProjectUncounted => {
            // A scan over every dormant rule that reports no counter: only the independent allocation measurement sees it.
            let scan = |total: usize| {
                std::hint::black_box((0..total).map(|rule| vec![rule]).collect::<Vec<_>>());
            };
            let small = measure_project(3, &[1], &|_| {});
            let honest = measure_project(TOTAL, &[129], &|_| {});
            let uncounted = measure_project(TOTAL, &[129], &scan);
            Answer::Uncounted {
                counters_unchanged: honest.1 == uncounted.1,
                matches: [
                    project_cost_matches(small, honest),
                    project_cost_matches(small, uncounted),
                ],
            }
        }
    };
    Ok((Cow::Owned(windows.into_inner()), answer))
}
/// A measurement that saw no activation work and no allocation.
const IDLE: Sample = Sample {
    work: [0; 3],
    rule_sums: [0; 2],
    graph_hits: 0,
    blocks: 0,
    bytes: 0,
};
const OPENED: Window = Window::opened(0);
const fn windows(windows: &'static [Window]) -> Cow<'static, [Window]> {
    Cow::Borrowed(windows)
}
const CASES: &[Case<Input, Value, OperatingError>] = &[
    Case {
        name: CaseName("same_131_rule_sparse_intent_reads_only_its_required_metadata"),
        word: Word::Succeeds,
        input: || Input::Sparse,
        expect: Expect::Value((windows(&[OPENED]), Answer::Work([1; 3]))),
    },
    Case {
        name: CaseName(
            "rootless_activation_has_zero_work_and_allocations_with_dense_dormant_metadata",
        ),
        word: Word::Succeeds,
        input: || Input::Empty,
        expect: Expect::Value((windows(&[OPENED, OPENED]), Answer::Samples([IDLE, IDLE]))),
    },
    Case {
        name: CaseName(
            "one_selector_has_identical_actual_work_and_allocations_with_128_dormant_roots",
        ),
        word: Word::Succeeds,
        input: || Input::Selector,
        expect: Expect::Value((
            windows(&[OPENED, OPENED]),
            Answer::Selector {
                work: [1; 3],
                graph_hits: 0,
                allocates: true,
                same: true,
            },
        )),
    },
    Case {
        name: CaseName("tail_graph_activation_probes_only_the_relevant_rule"),
        word: Word::Succeeds,
        input: || Input::Graph,
        expect: Expect::Value((
            windows(&[OPENED, OPENED]),
            Answer::Tail {
                work: [1, 3, 1],
                graph_hits: 1,
                rule_sums: [[1; 2], [129; 2]],
                same_cost: true,
                same: false,
            },
        )),
    },
    Case {
        name: CaseName("mixed_graph_and_selector_activation_scales_with_three_relevant_roots"),
        word: Word::Succeeds,
        input: || Input::Mixed,
        expect: Expect::Value((
            windows(&[OPENED, OPENED]),
            Answer::Tail {
                work: [3, 5, 2],
                graph_hits: 1,
                rule_sums: [[3, 1], [3, 1]],
                same_cost: true,
                same: true,
            },
        )),
    },
    Case {
        name: CaseName("noncontiguous_selector_intents_use_their_own_sorted_rule_ranges"),
        word: Word::Succeeds,
        input: || Input::TailSelectors,
        expect: Expect::Value((
            windows(&[OPENED, OPENED]),
            Answer::Tail {
                work: [2; 3],
                graph_hits: 0,
                rule_sums: [[2; 2], [130; 2]],
                same_cost: true,
                same: false,
            },
        )),
    },
    Case {
        name: CaseName(
            "actual_dense_activation_is_rejected_by_the_sparse_work_and_allocation_oracle",
        ),
        word: Word::Succeeds,
        input: || Input::Dense,
        expect: Expect::Value((
            windows(&[OPENED, OPENED]),
            Answer::Dense {
                work: [131, 261, 2],
                grows: true,
                oracle: [true, false],
            },
        )),
    },
    Case {
        name: CaseName(
            "project_roots_cost_the_same_actual_work_and_allocations_with_128_dormant_roots",
        ),
        word: Word::Succeeds,
        input: || Input::ProjectSparse,
        expect: Expect::Value((
            windows(&[Window::opened(2), Window::opened(2)]),
            Answer::ProjectSparse {
                accesses: [
                    Access {
                        reads: 5,
                        read_rules: 5,
                    },
                    Access {
                        reads: 5,
                        read_rules: 645,
                    },
                ],
                same_cost: true,
            },
        )),
    },
    Case {
        name: CaseName(
            "actual_dense_project_submission_is_rejected_by_the_sparse_work_and_allocation_oracle",
        ),
        word: Word::Succeeds,
        input: || Input::ProjectDense,
        expect: Expect::Value((
            windows(&[Window::opened(2), Window::opened(262)]),
            Answer::ProjectDense {
                reads: [5, 655],
                grows: true,
                oracle: [true, false],
            },
        )),
    },
    Case {
        name: CaseName("an_uncounted_dormant_project_scan_is_detected_by_allocation"),
        word: Word::Succeeds,
        input: || Input::ProjectUncounted,
        expect: Expect::Value((
            windows(&[Window::opened(2), Window::opened(2), Window::opened(2)]),
            Answer::Uncounted {
                counters_unchanged: true,
                matches: [true, false],
            },
        )),
    },
];
fn main() {
    // This target is alone in its process; assertions count blocks and bytes,
    // never elapsed time. Request compilation and full intent binding precede
    // the observed derived activation boundary and are outside this claim; the
    // project cases observe phase opening through the last root delivery.
    // nextest lists the cases first (`--list --format terse`), then runs each by its name.
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.iter().any(|argument| argument == "--list") {
        let listed = !arguments.iter().any(|argument| argument == "--ignored");
        for case in CASES.iter().filter(|_| listed) {
            println!("{}: test", case.name.0);
        }
    } else {
        let _profiler = dhat::Profiler::builder().testing().build();
        let selected = arguments
            .iter()
            .find(|argument| !argument.starts_with("--"))
            .cloned();
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
}
