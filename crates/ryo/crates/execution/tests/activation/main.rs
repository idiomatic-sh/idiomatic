//! Actual derived activation visits and allocator blocks exclude dormant compiled roots.
#[cfg(test)]
mod fixtures;
use fixtures::file::measure;
use fixtures::gated::{Operators, came_to, file_rules, warm_cost};
use fixtures::plans::{self, Given};
use fixtures::project::measure_project;
use fixtures::subscriber::{Access, Sample, Window, project_cost_matches, same_cost};
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::prepare::types::PrepareRequest;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;
use std::cell::RefCell;

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

#[derive(Clone, Copy)]
enum Input {
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
enum Answer {
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
type Value = (Cow<'static, [Window]>, Answer);
fn subject(input: Input) -> Result<Value, OperatingError> {
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
const fn windows(windows: &'static [Window]) -> Cow<'static, [Window]> {
    Cow::Borrowed(windows)
}
const CASES: &[Case<Input, Value, OperatingError>] = &[
    Case {
        name: CaseName("one_demanded_rule_allocates_the_same_beside_128_dormant_rules"),
        word: Word::Succeeds,
        input: || Input::RunSelector,
        expect: Expect::Value((windows(&[]), SAME)),
    },
    Case {
        name: CaseName("no_demanded_rule_allocates_the_same_beside_128_dormant_rules"),
        word: Word::Succeeds,
        input: || Input::RunEmpty,
        expect: Expect::Value((windows(&[]), NOTHING)),
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
                        reads: 3,
                        read_rules: 3,
                    },
                    Access {
                        reads: 3,
                        read_rules: 387,
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
                reads: [3, 393],
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
    Case {
        name: CaseName("a_rule_whose_root_kind_is_absent_allocates_as_its_root_alone"),
        word: Word::Succeeds,
        input: || Input::RootAbsent,
        expect: Expect::Value((windows(&[]), SAME)),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_antijoin_of_an_absent_kind"),
        word: Word::Succeeds,
        input: || Input::Gated(plans::antijoin_of_an_absent_kind, plans::no_facts),
        expect: Expect::Value((windows(&[]), gated("rule 0 complete 1; finished; reads []"))),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_union_with_an_absent_kind"),
        word: Word::Succeeds,
        input: || Input::Gated(plans::union_with_an_absent_kind, plans::no_facts),
        expect: Expect::Value((windows(&[]), gated("rule 0 complete 1; finished; reads []"))),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_shared_stage_one_branch_absent"),
        word: Word::Succeeds,
        input: || Input::Gated(plans::shared_stage_one_branch_absent, plans::no_facts),
        expect: Expect::Value((
            windows(&[]),
            gated("rule 0 complete 2; rule 1 complete 0; finished; reads []"),
        )),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_refused_literal_pattern"),
        word: Word::Succeeds,
        input: || {
            Input::Gated(
                plans::refused_literal_pattern_joined_with_an_absent_kind,
                plans::no_facts,
            )
        },
        expect: Expect::Value((
            windows(&[]),
            gated(
                "rule 0 failed InvalidRegex { reason: \"regex parse error:\\n    [\\n    ^\\nerror: unclosed character class\" }; unfinished; reads []",
            ),
        )),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_observer_of_a_dormant_producer"),
        word: Word::Succeeds,
        input: || Input::Gated(plans::observer_of_two_producers_one_absent, plans::no_facts),
        expect: Expect::Value((
            windows(&[]),
            gated("rule 0 complete 2; rule 1 complete 0; rule 2 complete 2; finished; reads []"),
        )),
    },
    Case {
        name: CaseName(
            "gated_and_all_active_runs_answer_the_same_functions_joined_with_an_absent_kind",
        ),
        word: Word::Succeeds,
        input: || {
            Input::Gated(
                plans::functions_joined_with_an_absent_kind,
                plans::two_functions,
            )
        },
        expect: Expect::Value((
            windows(&[]),
            gated("rule 0 complete 0; finished; reads [Function]"),
        )),
    },
    Case {
        name: CaseName(
            "gated_and_all_active_runs_answer_the_same_no_function_joined_with_an_absent_kind",
        ),
        word: Word::Succeeds,
        input: || {
            Input::Gated(
                plans::functions_joined_with_an_absent_kind,
                plans::no_function,
            )
        },
        expect: Expect::Value((
            windows(&[]),
            gated("rule 0 complete 0; finished; reads [Function]"),
        )),
    },
    Case {
        name: CaseName(
            "gated_and_all_active_runs_answer_the_same_refused_functions_read_by_a_dormant_join",
        ),
        word: Word::Succeeds,
        input: || Input::Gated(plans::absent_kind_joined_with_functions, plans::no_facts),
        expect: Expect::Value((
            windows(&[]),
            gated(
                "rule 0 withheld; rule 0 stopped Facts(Store { reason: \"Function was not given\" }); unfinished; reads [Function]",
            ),
        )),
    },
    Case {
        name: CaseName(
            "gated_and_all_active_runs_answer_the_same_functions_read_in_part_by_a_dormant_join",
        ),
        word: Word::Succeeds,
        input: || {
            Input::Gated(
                plans::absent_kind_joined_with_functions,
                plans::no_function_read_in_part,
            )
        },
        expect: Expect::Value((
            windows(&[]),
            gated(
                "rule 0 Partial { table: Table { kinds: [Node], cells: [], count: RowCount(0) }, affected: [Some(FileId(0))] }; finished; reads [Function]",
            ),
        )),
    },
    Case {
        name: CaseName(
            "gated_and_all_active_runs_answer_the_same_functions_read_by_two_readers_once",
        ),
        word: Word::Succeeds,
        input: || Input::Gated(plans::functions_read_twice, plans::two_functions),
        expect: Expect::Value((
            windows(&[]),
            gated("rule 0 complete 2; rule 1 complete 2; finished; reads [Function]"),
        )),
    },
    Case {
        name: CaseName(
            "gated_and_all_active_runs_answer_the_same_skipped_reader_beside_another_select",
        ),
        word: Word::Succeeds,
        input: || {
            Input::Gated(
                plans::skipped_reader_beside_another_select,
                plans::two_functions,
            )
        },
        expect: Expect::Value((
            windows(&[]),
            gated("rule 0 complete 0; rule 1 complete 2; finished; reads [Function]"),
        )),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_union_reads_in_execution_order"),
        word: Word::Succeeds,
        input: || {
            Input::Gated(
                plans::comments_then_functions,
                plans::functions_and_comments,
            )
        },
        expect: Expect::Value((
            windows(&[]),
            gated("rule 0 complete 3; finished; reads [Comment, Function]"),
        )),
    },
    Case {
        name: CaseName(
            "gated_and_all_active_runs_answer_the_same_union_refused_in_execution_order",
        ),
        word: Word::Succeeds,
        input: || Input::Gated(plans::comments_then_functions, plans::no_facts),
        expect: Expect::Value((
            windows(&[]),
            gated(
                "rule 0 withheld; rule 0 stopped Facts(Store { reason: \"Comment was not given\" }); unfinished; reads [Comment]",
            ),
        )),
    },
];
/// The gated run answered as the all-active run, its rules having come to `rules`.
const fn gated(rules: &'static str) -> Answer {
    Answer::Gated {
        same: true,
        rules: Cow::Borrowed(rules),
    }
}
/// What a whole run answers when dormant rules cost it nothing.
const SAME: Answer = Answer::RunSame {
    allocates: true,
    same: true,
};
/// What a whole run of no demanded rule answers: no rule runs on the file, so its sources are asked for nothing and the run
/// allocates nothing, beside dormant rules as alone. The meter is shown live by the run of one demanded rule.
const NOTHING: Answer = Answer::RunSame {
    allocates: false,
    same: true,
};
fn main() {
    // This target is alone in its process; assertions count blocks and bytes,
    // never elapsed time. Request compilation and full intent binding precede
    // the observed derived activation boundary and are outside this claim; the
    // project cases observe phase opening through the last root delivery.
    // libtest-mimic answers libtest's command line (`--list`, `--exact`, `--skip`, filters), one trial per case.
    let mut arguments = libtest_mimic::Arguments::from_args();
    // dhat counts the whole process's heap, so the trials run one at a time, on this thread.
    arguments.test_threads = Some(1);
    let profiler = (!arguments.list).then(|| dhat::Profiler::builder().testing().build());
    let trials = CASES
        .iter()
        .map(|case| {
            libtest_mimic::Trial::test(case.name.0, move || {
                run(std::slice::from_ref(case), Word::Succeeds, subject);
                Ok(())
            })
        })
        .collect();
    let conclusion = libtest_mimic::run(&arguments, trials);
    drop(profiler);
    // A run that names no filter, skip or listing executes a case: an empty table is refused.
    let unfiltered = !arguments.list
        && !arguments.ignored
        && arguments.filter.is_none()
        && arguments.skip.is_empty();
    assert!(
        !unfiltered || conclusion.num_passed + conclusion.num_failed > 0,
        "the activation target must execute an actual case"
    );
    conclusion.exit();
}
