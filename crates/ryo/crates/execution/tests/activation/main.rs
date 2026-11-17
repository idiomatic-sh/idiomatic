//! Actual derived activation visits and allocator blocks exclude dormant compiled roots.
#[cfg(test)]
mod fixtures;
use fixtures::compared::{Answer, Input, Value, subject};
use fixtures::given;
use fixtures::plans;
use fixtures::subscriber::{Access, Window};
use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use std::borrow::Cow;

#[global_allocator]
static ALLOCATOR: dhat::Alloc = dhat::Alloc;

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
        input: || Input::Gated(plans::antijoin_of_an_absent_kind, given::no_facts),
        expect: Expect::Value((windows(&[]), gated("rule 0 complete 1; finished; reads []"))),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_union_with_an_absent_kind"),
        word: Word::Succeeds,
        input: || Input::Gated(plans::union_with_an_absent_kind, given::no_facts),
        expect: Expect::Value((windows(&[]), gated("rule 0 complete 1; finished; reads []"))),
    },
    Case {
        name: CaseName("gated_and_all_active_runs_answer_the_same_shared_stage_one_branch_absent"),
        word: Word::Succeeds,
        input: || Input::Gated(plans::shared_stage_one_branch_absent, given::no_facts),
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
                given::no_facts,
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
        input: || Input::Gated(plans::observer_of_two_producers_one_absent, given::no_facts),
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
                given::two_functions,
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
                given::no_function,
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
        input: || Input::Gated(plans::absent_kind_joined_with_functions, given::no_facts),
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
                given::no_function_read_in_part,
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
        input: || Input::Gated(plans::functions_read_twice, given::two_functions),
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
                given::two_functions,
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
                given::functions_and_comments,
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
        input: || Input::Gated(plans::comments_then_functions, given::no_facts),
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
