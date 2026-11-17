//! What a case of the activation target runs and compares: whole-file allocation, project-phase work, and the gated run
//! against the run with every node active.
use super::file::measure;
use super::gated::{Operators, came_to, file_rules, warm_cost};
use super::given::Given;
use super::plans;
use super::project::measure_project;
use super::subscriber::{Access, Sample, Window, project_cost_matches, same_cost};
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use std::borrow::Cow;
use std::cell::RefCell;

/// What a case runs.
#[derive(Clone, Copy)]
pub(crate) enum Input {
    ProjectSparse,
    ProjectDense,
    ProjectUncounted,
    RunSelector,
    RunEmpty,
    /// A rule whose root kind the file lacks, with a step after its root, against the root alone.
    RootAbsent,
    /// A program run with the composed operators and with every node active.
    Gated(fn() -> PrepareRequest, fn() -> Given),
}
/// What a case compared, in counters (exact) and allocation relations (allocation sizes are the allocator's).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Answer {
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
    /// Whether the small whole run allocated, and whether the one beside dormant rules allocated exactly as much.
    RunSame { allocates: bool, same: bool },
    /// Whether the gated run's rules, file end and reads are the all-active run's (the gate removed at the runner), and what
    /// the gated run's rules came to with the relations it read.
    Gated {
        same: bool,
        rules: Cow<'static, str>,
    },
}
/// Every measurement's window, in order, and what the case compared.
pub(crate) type Value = (Cow<'static, [Window]>, Answer);
/// What `input` measured and compared.
pub(crate) fn subject(input: Input) -> Result<Value, OperatingError> {
    const TOTAL: usize = 131;
    let windows = RefCell::new(vec![]);
    // A whole file run's allocations, independent of how the engine reports its activation.
    let cost_of = |total: usize, demanded: &[usize]| measure(total, demanded).2;
    let measure_project = |total: usize, demanded: &[usize], intrusion: &dyn Fn(usize)| {
        let (sample, access, window) = measure_project(total, demanded, intrusion);
        windows.borrow_mut().push(window);
        (sample, access)
    };
    let answer = match input {
        Input::RootAbsent => {
            let alone = warm_cost(plans::absent_kind_alone());
            let joined = warm_cost(plans::absent_kind_joined());
            Answer::RunSame {
                allocates: alone.blocks > 0,
                same: alone.blocks == joined.blocks,
            }
        }
        Input::Gated(program, given) => {
            let (gated, reads) = file_rules(program(), Operators::Gated, &given());
            let (all, ungated_reads) = file_rules(program(), Operators::AllActive, &given());
            Answer::Gated {
                same: format!("{gated:?}") == format!("{all:?}") && reads == ungated_reads,
                rules: Cow::Owned(format!("{}; reads {reads:?}", came_to(&gated))),
            }
        }
        Input::RunSelector | Input::RunEmpty => {
            let demanded: &[usize] = if matches!(input, Input::RunSelector) {
                &[0]
            } else {
                &[]
            };
            let small = cost_of(3, demanded);
            let dormant = cost_of(TOTAL, demanded);
            Answer::RunSame {
                allocates: small.blocks > 0 && small.bytes > 0,
                same: small == dormant,
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
