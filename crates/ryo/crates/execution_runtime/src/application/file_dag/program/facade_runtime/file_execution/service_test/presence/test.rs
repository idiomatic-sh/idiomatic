// ignore: idiomatic.arch/file-length - the R1 contract cases of one design share one fixture program, one file harness and one probe; splitting would duplicate the harness.
//! A rule that cannot match a file is never instantiated there: its root publishes the empty, complete table its full run
//! would, no region only it reaches gets a slot or a reader count, and nothing that could have a row or a row error is
//! skipped. Language-free: fixture relations, fixture rows and compiled programs only.
use super::fixtures::routing::routed;
use super::fixtures::{
    Facts, Ran, TEXT, at, compare, equal, facts, join_on_file, plan, prepare, regions_of, rule_b,
    run as run_file, run_intent, select, step,
};
use crate::application::file_dag::program::facade_runtime::types::{
    InputRelease, Instantiated, RegionProbe,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::{Partial, RuleIndex};
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::rule_intent::types::{ApplicabilityReason, FileRuleIntent};
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value as CellValue};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as PlanInput, JoinColumn, LiteralText, Operand, QueryLiteral, RepeatedRows,
    Side, StepScope, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::stored_facts::types::Digest;
use std::borrow::Cow;
use std::sync::Arc;

/// What a rule's root answered on the file: its rows and their kinds, or the outcome's kind with
/// its error as the error prints.
#[derive(Clone, Debug, PartialEq)]
enum Outcome {
    Rows(usize, Cow<'static, [CellKind]>),
    Partial(usize),
    Skipped,
    Failed(Cow<'static, str>),
    Unknown(Cow<'static, str>),
    Refused(Cow<'static, str>),
    NoQuery,
    Deferred,
}

/// What one file run answered and left behind: each rule's outcome and the rows each of its steps
/// was fed, each probed region's instantiation and a second release's refusal, the roots presence
/// found absent, the cleanup refusal, the dormant obligations and the finish refusal.
#[derive(Clone, Debug, PartialEq)]
struct Observed {
    outcomes: Cow<'static, [Outcome]>,
    fed: Cow<'static, [Cow<'static, [u64]>]>,
    probes: Cow<'static, [RegionProbe]>,
    presence_absent: u64,
    again: Cow<'static, [Option<OperatingError>]>,
    cleanup: Option<OperatingError>,
    dormant: usize,
    finished: Option<OperatingError>,
}

impl From<Ran> for Observed {
    fn from(ran: Ran) -> Self {
        let printed = |error: OperatingError| Cow::Owned(format!("{error:?}"));
        Self {
            outcomes: ran
                .outcomes
                .into_iter()
                .map(|outcome| {
                    outcome.map_or_else(
                        |error| Outcome::Refused(printed(error)),
                        |outcome| match outcome {
                            RootOutcome::Complete(table) => {
                                Outcome::Rows(table.count.0, Cow::Owned(table.kinds))
                            }
                            RootOutcome::Partial { table, .. } => Outcome::Partial(table.count.0),
                            RootOutcome::Skipped(_) => Outcome::Skipped,
                            RootOutcome::Failed(error) => Outcome::Failed(printed(error)),
                            RootOutcome::Unknown(error) => Outcome::Unknown(printed(error)),
                            RootOutcome::NoQuery => Outcome::NoQuery,
                            RootOutcome::Deferred { .. } => Outcome::Deferred,
                        },
                    )
                })
                .collect(),
            fed: ran.fed.into_iter().map(Cow::Owned).collect(),
            probes: Cow::Owned(ran.probes),
            presence_absent: ran.presence_absent,
            again: ran.again.into_iter().map(Result::err).collect(),
            cleanup: ran.cleanup.err(),
            dormant: ran.dormant,
            finished: ran.finished.err(),
        }
    }
}

/// Runs every rule of `prepared` on one file and adds what it observed to `runs`.
fn run_into(
    runs: &mut Vec<Observed>,
    prepared: &Arc<PreparedProgram>,
    facts: &mut Facts,
    intents: &[FileRuleIntent],
    partial: &[Partial],
    regions: &[RegionId],
) {
    runs.push(Observed::from(run_file(
        prepared, facts, intents, partial, regions,
    )));
}

/// A fact of the compiled program a row also states.
#[derive(Debug, PartialEq)]
enum Fact {
    /// Whether B's root is a region of its own, apart from every dependency.
    OwnRootRegion(bool),
    /// The regions of the probed rule's steps.
    Regions(usize),
    /// The relation columns whose provider spans presence relies on.
    Spans(Cow<'static, [(Relation, Cow<'static, [ColumnIndex]>)]>),
    /// Whether the program compiles a region onto the ordered-existence route.
    Routed(bool),
}

/// What a design case observed.
#[derive(Debug, PartialEq)]
enum Seen {
    /// Each file run, in order.
    Runs(Cow<'static, [Observed]>),
    /// A fact of the program, and each file run.
    Program {
        fact: Fact,
        runs: Cow<'static, [Observed]>,
    },
    /// Whether each consumer step's presence is always.
    Always(Cow<'static, [bool]>),
}

/// The design case a row compiles and runs.
#[derive(Clone, Copy, Debug)]
enum Design {
    /// B absent on a file where its comment test fails, and a control file where its join drops every row.
    AbsentRule,
    /// An antijoin and a union over a file where both have rows.
    AbsenceAndAlternative,
    /// A self join tested on each side, over a file with both texts in different rows.
    SelfJoinLiterals,
    /// A join with no view, where both sides have rows and where its right side has none.
    JoinWithoutView,
    /// Row-fallible operators upstream: duplicate order, span bounds, and Replace over Symbol and Node columns.
    RowErrorsUpstream,
    /// A source search over provider spans, on a file with valid spans and one with a span outside the source.
    ReliedSpan,
    /// A Replace whose pattern does not compile.
    InvalidReplacePattern,
    /// A select whose `matches` pattern does not compile, by syntax and by size.
    InvalidSelectPatterns,
    /// An absent root read by an admitted rule, which reads the root's input or not.
    AbsentRootInsideAdmitted,
    /// An absent root read by an absent interior join of an admitted rule.
    CascadeStop,
    /// A program with an ordered-existence route, on an empty file.
    RoutedAbsentRoot,
    /// An absent rule sharing a region with a rule that ran, and that rule alone.
    Counts,
    /// B inapplicable on a file, and B on a file with a partial read.
    InapplicableAndPartial,
    /// A comparison that errs on a row the pipeline drops before it.
    TestErrorDropped,
    /// A step over an invalid schema whose formula is false.
    SchemaErrorUnderFalseFormula,
    /// The consumers of a producer's bound observation output.
    BoundObservationConsumers,
}

type Input = Design;
type Value = Seen;
type Failure = OperatingError;

/// The kinds of a comment row.
const COMMENT: Cow<'static, [CellKind]> = Cow::Borrowed(&[
    CellKind::Node,
    CellKind::Symbol,
    CellKind::File,
    CellKind::Span,
    CellKind::Flag,
]);

/// A file run that answered these outcomes from these fed rows, with this many roots found
/// absent, probing no region and leaving nothing owed or refused.
const fn clean(
    outcomes: &'static [Outcome],
    fed: &'static [Cow<'static, [u64]>],
    presence_absent: u64,
) -> Observed {
    probed(outcomes, fed, &[], presence_absent, &[])
}

/// A file run that answered these outcomes from these fed rows, left these probed regions and
/// second-release refusals, with this many roots found absent, and nothing owed or refused.
const fn probed(
    outcomes: &'static [Outcome],
    fed: &'static [Cow<'static, [u64]>],
    probes: &'static [RegionProbe],
    presence_absent: u64,
    again: &'static [Option<OperatingError>],
) -> Observed {
    Observed {
        outcomes: Cow::Borrowed(outcomes),
        fed: Cow::Borrowed(fed),
        probes: Cow::Borrowed(probes),
        presence_absent,
        again: Cow::Borrowed(again),
        cleanup: None,
        dormant: 0,
        finished: None,
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_absent_rule_is_never_instantiated_and_answers_as_its_full_run"),
        word: Word::Succeeds,
        input: || Design::AbsentRule,
        expect: Expect::Value(Seen::Program {
            fact: Fact::OwnRootRegion(true),
            runs: Cow::Borrowed(&[
                probed(
                    &[Outcome::Rows(0, COMMENT), Outcome::Rows(0, COMMENT)],
                    &[Cow::Borrowed(&[0, 0, 0]), Cow::Borrowed(&[0, 0, 0, 0, 0])],
                    &[
                        RegionProbe {
                            instantiated: Instantiated::Never,
                            inputs: None,
                        },
                        RegionProbe {
                            instantiated: Instantiated::Never,
                            inputs: None,
                        },
                        RegionProbe {
                            instantiated: Instantiated::Slot,
                            inputs: Some(InputRelease::Released),
                        },
                    ],
                    2,
                    &[
                        Some(OperatingError::NotRun),
                        Some(OperatingError::NotRun),
                        Some(OperatingError::NotRun),
                    ],
                ),
                probed(
                    &[Outcome::Rows(0, COMMENT), Outcome::Rows(0, COMMENT)],
                    &[Cow::Borrowed(&[0, 0, 0]), Cow::Borrowed(&[0, 0, 0, 0, 0])],
                    &[
                        RegionProbe {
                            instantiated: Instantiated::Never,
                            inputs: None,
                        },
                        RegionProbe {
                            instantiated: Instantiated::Never,
                            inputs: None,
                        },
                        RegionProbe {
                            instantiated: Instantiated::Slot,
                            inputs: Some(InputRelease::Released),
                        },
                    ],
                    2,
                    &[
                        Some(OperatingError::NotRun),
                        Some(OperatingError::NotRun),
                        Some(OperatingError::NotRun),
                    ],
                ),
            ]),
        }),
    },
    Case {
        name: CaseName("absences_and_alternatives_are_never_skipped_where_they_have_rows"),
        word: Word::Succeeds,
        input: || Design::AbsenceAndAlternative,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[clean(
            &[Outcome::Rows(1, COMMENT), Outcome::Rows(1, COMMENT)],
            &[Cow::Borrowed(&[1, 0, 1]), Cow::Borrowed(&[0, 1, 1])],
            2,
        )]))),
    },
    Case {
        name: CaseName("a_self_join_with_a_literal_on_each_side_runs"),
        word: Word::Succeeds,
        input: || Design::SelfJoinLiterals,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[clean(
            &[Outcome::Rows(
                1,
                Cow::Borrowed(&[CellKind::Symbol, CellKind::Symbol]),
            )],
            &[Cow::Borrowed(&[2, 6, 4])],
            0,
        )]))),
    },
    Case {
        name: CaseName("a_join_without_a_view_runs_where_both_sides_have_rows"),
        word: Word::Succeeds,
        input: || Design::JoinWithoutView,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[
            clean(
                &[Outcome::Rows(1, Cow::Borrowed(&[CellKind::Symbol]))],
                &[Cow::Borrowed(&[1, 1, 2])],
                0,
            ),
            clean(
                &[Outcome::Rows(0, Cow::Borrowed(&[CellKind::Symbol]))],
                &[Cow::Borrowed(&[0, 0, 0])],
                1,
            ),
        ]))),
    },
    Case {
        name: CaseName("row_errors_upstream_keep_the_rule_running"),
        word: Word::Succeeds,
        input: || Design::RowErrorsUpstream,
        expect: Expect::Value(Seen::Program {
            fact: Fact::Regions(4),
            runs: Cow::Borrowed(&[Observed {
                outcomes: Cow::Borrowed(&[
                    Outcome::Failed(Cow::Borrowed("DuplicateOrder")),
                    Outcome::Failed(Cow::Borrowed("InvalidSpanBounds")),
                    Outcome::Rows(0, Cow::Borrowed(&[CellKind::Node])),
                    Outcome::Failed(Cow::Borrowed("ShapeMismatch")),
                ]),
                fed: Cow::Borrowed(&[
                    Cow::Borrowed(&[2, 2, 0, 0, 0]),
                    Cow::Borrowed(&[2, 1, 0, 0]),
                    Cow::Borrowed(&[2, 0, 0, 0]),
                    Cow::Borrowed(&[2, 1, 0, 0]),
                ]),
                probes: Cow::Borrowed(&[
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Dormant,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                ]),
                presence_absent: 2,
                again: Cow::Borrowed(&[
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                ]),
                cleanup: None,
                dormant: 1,
                finished: None,
            }]),
        }),
    },
    Case {
        name: CaseName("a_relied_span_outside_the_source_turns_presence_off"),
        word: Word::Succeeds,
        input: || Design::ReliedSpan,
        expect: Expect::Value(Seen::Program {
            fact: Fact::Spans(Cow::Borrowed(&[(
                Relation::Comment,
                Cow::Borrowed(&[ColumnIndex(3)]),
            )])),
            runs: Cow::Borrowed(&[
                clean(
                    &[Outcome::Rows(0, Cow::Borrowed(&[CellKind::Node]))],
                    &[Cow::Borrowed(&[0, 0, 0])],
                    1,
                ),
                Observed {
                    outcomes: Cow::Borrowed(&[Outcome::Failed(Cow::Borrowed(
                        "InvalidSourceMatchSpan",
                    ))]),
                    fed: Cow::Borrowed(&[Cow::Borrowed(&[2, 0, 0])]),
                    probes: Cow::Borrowed(&[]),
                    presence_absent: 0,
                    again: Cow::Borrowed(&[]),
                    cleanup: None,
                    dormant: 1,
                    finished: None,
                },
            ]),
        }),
    },
    Case {
        name: CaseName("an_invalid_replace_pattern_is_reported_where_its_input_has_rows"),
        word: Word::Succeeds,
        input: || Design::InvalidReplacePattern,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[Observed {
            outcomes: Cow::Borrowed(&[Outcome::Failed(Cow::Borrowed(
                "InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }",
            ))]),
            fed: Cow::Borrowed(&[Cow::Borrowed(&[1, 1, 0, 0])]),
            probes: Cow::Borrowed(&[]),
            presence_absent: 0,
            again: Cow::Borrowed(&[]),
            cleanup: None,
            dormant: 1,
            finished: None,
        }]))),
    },
    Case {
        name: CaseName("an_invalid_select_pattern_is_reported_where_its_input_has_rows"),
        word: Word::Succeeds,
        input: || Design::InvalidSelectPatterns,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[
            Observed {
                outcomes: Cow::Borrowed(&[Outcome::Failed(Cow::Borrowed(
                    "InvalidRegex { reason: \"regex parse error:\\n    (\\n    ^\\nerror: unclosed group\" }",
                ))]),
                fed: Cow::Borrowed(&[Cow::Borrowed(&[1, 1, 0, 0])]),
                probes: Cow::Borrowed(&[]),
                presence_absent: 0,
                again: Cow::Borrowed(&[]),
                cleanup: None,
                dormant: 1,
                finished: None,
            },
            Observed {
                outcomes: Cow::Borrowed(&[Outcome::Failed(Cow::Borrowed(
                    "InvalidRegex { reason: \"error building NFA\" }",
                ))]),
                fed: Cow::Borrowed(&[Cow::Borrowed(&[1, 1, 0, 0])]),
                probes: Cow::Borrowed(&[]),
                presence_absent: 0,
                again: Cow::Borrowed(&[]),
                cleanup: None,
                dormant: 1,
                finished: None,
            },
        ]))),
    },
    Case {
        name: CaseName("an_absent_root_inside_an_admitted_rule_leaves_nothing_owed"),
        word: Word::Succeeds,
        input: || Design::AbsentRootInsideAdmitted,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[
            probed(
                &[Outcome::Rows(0, COMMENT), Outcome::Rows(1, COMMENT)],
                &[
                    Cow::Borrowed(&[0, 0, 0, 0]),
                    Cow::Borrowed(&[0, 0, 0, 0, 1, 1]),
                ],
                &[
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                ],
                1,
                &[
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                ],
            ),
            probed(
                &[Outcome::Rows(0, COMMENT), Outcome::Rows(1, COMMENT)],
                &[
                    Cow::Borrowed(&[1, 0, 0, 0]),
                    Cow::Borrowed(&[1, 0, 0, 0, 0, 1]),
                ],
                &[
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                ],
                1,
                &[
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                ],
            ),
        ]))),
    },
    Case {
        name: CaseName("a_cascade_stops_at_an_absent_root_that_never_ran"),
        word: Word::Succeeds,
        input: || Design::CascadeStop,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[Observed {
            outcomes: Cow::Borrowed(&[Outcome::Rows(1, COMMENT), Outcome::Rows(0, COMMENT)]),
            fed: Cow::Borrowed(&[
                Cow::Borrowed(&[0, 0, 0, 0, 0, 0, 1, 1]),
                Cow::Borrowed(&[0, 0, 0, 0]),
            ]),
            probes: Cow::Borrowed(&[
                RegionProbe {
                    instantiated: Instantiated::Never,
                    inputs: None,
                },
                RegionProbe {
                    instantiated: Instantiated::Never,
                    inputs: None,
                },
                RegionProbe {
                    instantiated: Instantiated::Never,
                    inputs: None,
                },
                RegionProbe {
                    instantiated: Instantiated::Slot,
                    inputs: Some(InputRelease::Released),
                },
            ]),
            presence_absent: 2,
            again: Cow::Borrowed(&[
                Some(OperatingError::NotRun),
                Some(OperatingError::NotRun),
                Some(OperatingError::NotRun),
                Some(OperatingError::NotRun),
            ]),
            cleanup: None,
            dormant: 1,
            finished: None,
        }]))),
    },
    Case {
        name: CaseName("an_absent_root_of_a_routed_program_never_enters_the_route"),
        word: Word::Succeeds,
        input: || Design::RoutedAbsentRoot,
        expect: Expect::Value(Seen::Program {
            fact: Fact::Routed(true),
            runs: Cow::Borrowed(&[probed(
                &[Outcome::Rows(
                    0,
                    Cow::Borrowed(&[
                        CellKind::Node,
                        CellKind::File,
                        CellKind::Span,
                        CellKind::Number,
                        CellKind::Number,
                        CellKind::Number,
                        CellKind::Number,
                        CellKind::Number,
                        CellKind::Number,
                    ]),
                )],
                &[Cow::Borrowed(&[0, 0, 0, 0, 0])],
                &[
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                ],
                1,
                &[
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                ],
            )]),
        }),
    },
    Case {
        name: CaseName("counts_report_only_what_ran"),
        word: Word::Succeeds,
        input: || Design::Counts,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[
            clean(
                &[Outcome::Rows(0, COMMENT), Outcome::Rows(1, COMMENT)],
                &[Cow::Borrowed(&[1, 0, 0]), Cow::Borrowed(&[1, 1])],
                1,
            ),
            clean(&[Outcome::Rows(1, COMMENT)], &[Cow::Borrowed(&[1, 1])], 0),
        ]))),
    },
    Case {
        name: CaseName("inapplicable_rules_and_partial_files_keep_their_full_routes"),
        word: Word::Succeeds,
        input: || Design::InapplicableAndPartial,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[
            probed(
                &[Outcome::Refused(Cow::Borrowed("NotRun"))],
                &[Cow::Borrowed(&[])],
                &[
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                    RegionProbe {
                        instantiated: Instantiated::Never,
                        inputs: None,
                    },
                ],
                0,
                &[
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                ],
            ),
            probed(
                &[Outcome::Rows(0, COMMENT)],
                &[Cow::Borrowed(&[1, 1, 0])],
                &[
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                    RegionProbe {
                        instantiated: Instantiated::Slot,
                        inputs: Some(InputRelease::Released),
                    },
                ],
                0,
                &[
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                    Some(OperatingError::NotRun),
                ],
            ),
        ]))),
    },
    Case {
        name: CaseName("a_test_error_holds_and_the_answer_is_unchanged"),
        word: Word::Succeeds,
        input: || Design::TestErrorDropped,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[clean(
            &[Outcome::Rows(0, COMMENT)],
            &[Cow::Borrowed(&[0, 0])],
            1,
        )]))),
    },
    Case {
        name: CaseName("a_schema_error_under_a_false_formula_is_reported"),
        word: Word::Succeeds,
        input: || Design::SchemaErrorUnderFalseFormula,
        expect: Expect::Value(Seen::Runs(Cow::Borrowed(&[clean(
            &[Outcome::Failed(Cow::Borrowed(
                "Table(NoSuchColumn { index: 0 })",
            ))],
            &[Cow::Borrowed(&[0, 0, 0])],
            1,
        )]))),
    },
    Case {
        name: CaseName("consumers_of_a_bound_observation_are_never_absent"),
        word: Word::Succeeds,
        input: || Design::BoundObservationConsumers,
        expect: Expect::Value(Seen::Always(Cow::Borrowed(&[true, true, false, true]))),
    },
];

/// Compiles the design case's program and runs its files.
fn subject(design: Input) -> Result<Value, Failure> {
    Ok(match design {
        Design::AbsentRule => an_absent_rule_is_never_instantiated_and_answers_as_its_full_run(),
        Design::AbsenceAndAlternative => {
            absences_and_alternatives_are_never_skipped_where_they_have_rows()
        }
        Design::SelfJoinLiterals => a_self_join_with_a_literal_on_each_side_runs(),
        Design::JoinWithoutView => a_join_without_a_view_runs_where_both_sides_have_rows(),
        Design::RowErrorsUpstream => row_errors_upstream_keep_the_rule_running(),
        Design::ReliedSpan => a_relied_span_outside_the_source_turns_presence_off(),
        Design::InvalidReplacePattern => {
            an_invalid_replace_pattern_is_reported_where_its_input_has_rows()
        }
        Design::InvalidSelectPatterns => {
            an_invalid_select_pattern_is_reported_where_its_input_has_rows()
        }
        Design::AbsentRootInsideAdmitted => {
            an_absent_root_inside_an_admitted_rule_leaves_nothing_owed()
        }
        Design::CascadeStop => a_cascade_stops_at_an_absent_root_that_never_ran(),
        Design::RoutedAbsentRoot => an_absent_root_of_a_routed_program_never_enters_the_route(),
        Design::Counts => counts_report_only_what_ran(),
        Design::InapplicableAndPartial => {
            inapplicable_rules_and_partial_files_keep_their_full_routes()
        }
        Design::TestErrorDropped => a_test_error_holds_and_the_answer_is_unchanged(),
        Design::SchemaErrorUnderFalseFormula => a_schema_error_under_a_false_formula_is_reported(),
        Design::BoundObservationConsumers => consumers_of_a_bound_observation_are_never_absent(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

/// R1.1 and R1.2: B is absent. Its root has one slot, published and `Consumed`; no dependency of B has a slot or a reader
/// count; cleanup records nothing. Its answer equals the full run's on a control file where B's formula holds but its join
/// drops every row, and equals an antijoin's empty-input gate over the same shape there.
fn an_absent_rule_is_never_instantiated_and_answers_as_its_full_run() -> Seen {
    let mut runs = Vec::new();
    let gate = plan(vec![
        select(
            PlanInput::Relation(Relation::Comment),
            vec![equal(TEXT, "y")],
        ),
        select(PlanInput::Relation(Relation::Function), vec![]),
        join_on_file(0, 1, 5),
        select(PlanInput::Relation(Relation::Comment), vec![]),
        step(Operation::Antijoin {
            left: at(2),
            right: at(3),
            left_on: vec![ColumnIndex(0)],
            right_on: vec![ColumnIndex(0)],
        }),
    ]);
    let prepared = prepare(vec![rule_b(), gate]);
    let b = regions_of(&prepared, 0);
    let intents = [run_intent(), run_intent()];
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &intents,
        &[],
        &b,
    );
    let (root, dependencies) = b.split_last().expect("B has steps");
    let own_region = dependencies.iter().all(|region| region != root);
    // The control file: a "y" comment, so B's formula holds, but no function, so the join drops every row.
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["y"], &[]),
        &intents,
        &[],
        &b,
    );
    // Negative control: on the control file B's root ran in full and released its inputs; a second
    // release still refuses.
    Seen::Program {
        fact: Fact::OwnRootRegion(own_region),
        runs: Cow::Owned(runs),
    }
}

/// R1.3: rules whose answer is an absence or an alternative are not skipped where they have rows.
fn absences_and_alternatives_are_never_skipped_where_they_have_rows() -> Seen {
    let mut runs = Vec::new();
    let antijoin = plan(vec![
        select(
            PlanInput::Relation(Relation::Comment),
            vec![equal(TEXT, "x")],
        ),
        select(
            PlanInput::Relation(Relation::Function),
            vec![equal(TEXT, "y")],
        ),
        step(Operation::Antijoin {
            left: at(0),
            right: at(1),
            left_on: vec![ColumnIndex(2)],
            right_on: vec![ColumnIndex(2)],
        }),
    ]);
    let union = plan(vec![
        select(
            PlanInput::Relation(Relation::Comment),
            vec![equal(TEXT, "y")],
        ),
        select(
            PlanInput::Relation(Relation::Comment),
            vec![equal(TEXT, "x")],
        ),
        step(Operation::Union {
            left: at(0),
            right: at(1),
        }),
    ]);
    let prepared = prepare(vec![antijoin, union]);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent(), run_intent()],
        &[],
        &[],
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.4: `l.text = "x" and r.text = "z"` over a self join, on a file with x and z in different rows, runs and answers.
fn a_self_join_with_a_literal_on_each_side_runs() -> Seen {
    let mut runs = Vec::new();
    let self_join = plan(vec![
        select(PlanInput::Relation(Relation::Comment), vec![]),
        step(Operation::Join {
            left: at(0),
            right: at(0),
            left_on: vec![ColumnIndex(2)],
            right_on: vec![ColumnIndex(2)],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(TEXT),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(TEXT),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
        select(at(1), vec![equal(0, "x"), equal(1, "z")]),
    ]);
    let prepared = prepare(vec![self_join]);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x", "z"], &[]),
        &[run_intent()],
        &[],
        &[],
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.5: a join with no view runs where both sides have rows and is absent where its right side has none. How a filter,
/// a view and a reach join resolve their right columns is pinned by the fold's own cases.
fn a_join_without_a_view_runs_where_both_sides_have_rows() -> Seen {
    let mut runs = Vec::new();
    let joined = plan(vec![
        select(PlanInput::Relation(Relation::Comment), vec![]),
        select(
            PlanInput::Relation(Relation::Function),
            vec![equal(TEXT, "f")],
        ),
        step(Operation::Join {
            left: at(0),
            right: at(1),
            left_on: vec![ColumnIndex(2)],
            right_on: vec![ColumnIndex(2)],
            output: vec![JoinColumn {
                side: Side::Right,
                column: ColumnIndex(TEXT),
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
    ]);
    let prepared = prepare(vec![joined]);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["g"]),
        &[run_intent()],
        &[],
        &[],
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.6: a row-fallible operator upstream keeps the rule running where its input has rows, so its row error is reported
/// as in the full run; Replace over a Symbol column is no such operator, Replace over a Node column is.
fn row_errors_upstream_keep_the_rule_running() -> Seen {
    let mut runs = Vec::new();
    let absent = || {
        select(
            PlanInput::Relation(Relation::Function),
            vec![equal(TEXT, "y")],
        )
    };
    let ordered = plan(vec![
        select(PlanInput::Relation(Relation::Comment), vec![]),
        step(Operation::Extend {
            from: at(0),
            values: vec![Operand::Literal(QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(1),
            ))],
        }),
        absent(),
        step(Operation::Extend {
            from: at(2),
            values: vec![Operand::Literal(QueryLiteral::Number(
                ryo_relations::domain::relations::types::Integer(1),
            ))],
        }),
        step(Operation::OrderedMatch {
            left: at(1),
            right: at(3),
            left_on: vec![],
            right_on: vec![],
            left_order: ColumnIndex(5),
            right_order: ColumnIndex(4),
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            }],
        }),
    ]);
    let bounds = plan(vec![
        select(PlanInput::Relation(Relation::Comment), vec![]),
        step(Operation::SpanBounds {
            from: at(0),
            span: ColumnIndex(3),
        }),
        absent(),
        join_on_file(1, 2, 1),
    ]);
    let replace = |column: u32| {
        plan(vec![
            select(PlanInput::Relation(Relation::Comment), vec![]),
            step(Operation::Replace {
                from: at(0),
                text: ColumnIndex(column),
                pattern: LiteralText("a".into()),
                replacement: LiteralText("b".into()),
            }),
            absent(),
            join_on_file(1, 2, 1),
        ])
    };
    let prepared = prepare(vec![ordered, bounds, replace(TEXT), replace(0)]);
    let mut file = facts(&["x", "x"], &["f"]);
    if let Some(cell) = file.comment.cells.get_mut(3) {
        *cell = CellValue::Span(ryo_relations::domain::relations::constants::NO_SPAN);
    }
    let intents = [run_intent(), run_intent(), run_intent(), run_intent()];
    let symbol = regions_of(&prepared, 2);
    run_into(&mut runs, &prepared, &mut file, &intents, &[], &symbol);
    Seen::Program {
        fact: Fact::Regions(symbol.len()),
        runs: Cow::Owned(runs),
    }
}

/// L-span L3: a source search over a provider span declares no failure, so a rule can be absent where its search has
/// rows. The file validates the relied spans when presence first reads the relation: on a file whose comment span lies
/// outside the source, presence turns off and the rule reports the search's refusal as its full run does; on a file whose
/// spans are valid, the rule is absent and answers its full run's empty table.
fn a_relied_span_outside_the_source_turns_presence_off() -> Seen {
    let mut runs = Vec::new();
    let searched = plan(vec![
        step(Operation::SourceMatches {
            from: PlanInput::Relation(Relation::Comment),
            span: ColumnIndex(3),
            base_line: None,
            pattern: LiteralText("a".into()),
            capture: None,
            capture_text: ryo_values::domain::query_operators::types::TextOutput::Omitted,
            capture_line: ryo_values::domain::query_operators::types::LineOutput::Omitted,
        }),
        select(
            PlanInput::Relation(Relation::Function),
            vec![equal(TEXT, "y")],
        ),
        join_on_file(0, 1, 1),
    ]);
    let prepared = prepare(vec![searched]);
    let spans = prepared
        .program()
        .presence()
        .spans
        .iter()
        .map(|check| (check.relation, Cow::Owned(check.columns.to_vec())))
        .collect();
    // The source is empty: the first comment's span is at 0, inside it; the second's at 1, outside it.
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x", "x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    Seen::Program {
        fact: Fact::Spans(spans),
        runs: Cow::Owned(runs),
    }
}

/// R1.6: a literal pattern that does not compile is refused when a row first reaches it. A Replace over a Symbol column
/// cannot fail on its rows, but with such a pattern its input is fallible, so the rule runs where the input has rows and
/// reports the refusal as its full run does, even where its other conjunct cannot match.
fn an_invalid_replace_pattern_is_reported_where_its_input_has_rows() -> Seen {
    let mut runs = Vec::new();
    let invalid = plan(vec![
        select(PlanInput::Relation(Relation::Comment), vec![]),
        step(Operation::Replace {
            from: at(0),
            text: ColumnIndex(TEXT),
            pattern: LiteralText("(".into()),
            replacement: LiteralText("b".into()),
        }),
        select(
            PlanInput::Relation(Relation::Function),
            vec![equal(TEXT, "y")],
        ),
        join_on_file(1, 2, 1),
    ]);
    let prepared = prepare(vec![invalid]);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.6: a select's literal `matches` pattern is compiled when a row first reaches it, so one that does not compile, by
/// its syntax or by the compiled size limit, makes the select's input fallible: the rule runs where that input has rows
/// and reports the refusal as its full run does, even where its other conjunct cannot match.
fn an_invalid_select_pattern_is_reported_where_its_input_has_rows() -> Seen {
    let mut runs = Vec::new();
    for pattern in ["(", r"\w{1000}{1000}"] {
        let invalid = plan(vec![
            select(PlanInput::Relation(Relation::Comment), vec![]),
            select(at(0), vec![compare(TEXT, Test::Matches, pattern)]),
            select(
                PlanInput::Relation(Relation::Function),
                vec![equal(TEXT, "y")],
            ),
            join_on_file(1, 2, 1),
        ]);
        let prepared = prepare(vec![invalid]);
        run_into(
            &mut runs,
            &prepared,
            &mut facts(&["x"], &["f"]),
            &[run_intent()],
            &[],
            &[],
        );
    }
    Seen::Runs(Cow::Owned(runs))
}

/// R1.7: rule A's absent root reads X, which reads Y; A is also an input of admitted rule F. X and Y are never visited and
/// nothing is left owed at the end; when F also reads Y, Y runs once, for F.
fn an_absent_root_inside_an_admitted_rule_leaves_nothing_owed() -> Seen {
    let mut runs = Vec::new();
    // Y, X (a self join, its own region), the absent conjunct, and A's root joining X to it.
    let a = || {
        vec![
            select(PlanInput::Relation(Relation::Comment), vec![]),
            join_on_file(0, 0, 5),
            select(
                PlanInput::Relation(Relation::Function),
                vec![equal(TEXT, "y")],
            ),
            join_on_file(1, 2, 5),
        ]
    };
    for shared in [false, true] {
        let mut f = a();
        f.push(select(
            PlanInput::Relation(Relation::Comment),
            vec![equal(TEXT, "x")],
        ));
        f.push(step(Operation::Union {
            left: at(3),
            right: at(if shared { 0 } else { 4 }),
        }));
        let prepared = prepare(vec![plan(a()), plan(f)]);
        let regions = regions_of(&prepared, 0);
        let intents = [run_intent(), run_intent()];
        run_into(
            &mut runs,
            &prepared,
            &mut facts(&["x"], &["f"]),
            &intents,
            &[],
            &regions,
        );
    }
    Seen::Runs(Cow::Owned(runs))
}

/// R1.7, a never-polled absent root: rule A's absent root is read by an interior join of admitted rule F that is itself
/// absent on the file, so the join publishes without polling A and its release reaches A untouched. The cascade stops at
/// A: none of A's uncounted inputs is released, and nothing is refused.
fn a_cascade_stops_at_an_absent_root_that_never_ran() -> Seen {
    let mut runs = Vec::new();
    let a = || {
        vec![
            select(PlanInput::Relation(Relation::Comment), vec![]),
            join_on_file(0, 0, 5),
            select(
                PlanInput::Relation(Relation::Function),
                vec![equal(TEXT, "y")],
            ),
            join_on_file(1, 2, 5),
        ]
    };
    let mut f = a();
    f.push(select(
        PlanInput::Relation(Relation::Function),
        vec![equal(TEXT, "g")],
    ));
    f.push(join_on_file(3, 4, 5));
    f.push(select(
        PlanInput::Relation(Relation::Comment),
        vec![equal(TEXT, "x")],
    ));
    f.push(step(Operation::Union {
        left: at(5),
        right: at(6),
    }));
    let prepared = prepare(vec![plan(f), plan(a())]);
    let regions = regions_of(&prepared, 1);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent(), run_intent()],
        &[],
        &regions,
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.8: a program with an ordered-existence route publishes its absent root without entering the route or demanding any
/// of its inputs.
fn an_absent_root_of_a_routed_program_never_enters_the_route() -> Seen {
    let mut runs = Vec::new();
    let prepared = prepare(vec![routed()]);
    let regions = regions_of(&prepared, 0);
    // Whether the fixture compiles a routed region.
    let routed = regions.iter().any(|region| {
        matches!(
            prepared.program().regions()[region.0].route(),
            ryo_execution_domain::domain::executable_program::types::Route::OrderedExistence(_)
        )
    });
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&[], &[]),
        &[run_intent()],
        &[],
        &regions,
    );
    Seen::Program {
        fact: Fact::Routed(routed),
        runs: Cow::Owned(runs),
    }
}

/// R1.9: counts report what ran: an absent rule's steps report zero where their region did not run on the file, and the
/// count of a region it shares with a rule that ran; the other rule's counts are unchanged.
fn counts_report_only_what_ran() -> Seen {
    let mut runs = Vec::new();
    let shared = || select(PlanInput::Relation(Relation::Comment), vec![]);
    let absent = plan(vec![
        shared(),
        select(
            PlanInput::Relation(Relation::Function),
            vec![equal(TEXT, "y")],
        ),
        join_on_file(0, 1, 5),
    ]);
    let ran_rule = plan(vec![shared(), select(at(0), vec![equal(TEXT, "x")])]);
    let prepared = prepare(vec![absent, ran_rule]);
    let intents = [run_intent(), run_intent()];
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &intents,
        &[],
        &[],
    );
    run_into(
        &mut runs,
        &prepare(vec![plan(vec![
            shared(),
            select(at(0), vec![equal(TEXT, "x")]),
        ])]),
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        &[],
        &[],
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.10: an inapplicable rule keeps its skipped root and is never consulted; a file with a partial read runs with the gate
/// off, every root in full.
fn inapplicable_rules_and_partial_files_keep_their_full_routes() -> Seen {
    let mut runs = Vec::new();
    let prepared = prepare(vec![rule_b()]);
    let b = regions_of(&prepared, 0);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[FileRuleIntent::Skipped(ApplicabilityReason::SyntaxRejected)],
        &[],
        &b,
    );
    let partial = Partial {
        relation: Relation::Function,
        cause: ryo_values::domain::stored_facts::types::PartialCause("unparsed".into()),
        lacking: ryo_execution_domain::domain::coverage::types::Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(0)),
    };
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &["f"]),
        &[run_intent()],
        std::slice::from_ref(&partial),
        &b,
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.11: a test whose comparison errs on a row the pipeline drops before it holds; the rule's answer equals its full
/// run's.
fn a_test_error_holds_and_the_answer_is_unchanged() -> Seen {
    let mut runs = Vec::new();
    let erring = plan(vec![
        select(
            PlanInput::Relation(Relation::Comment),
            vec![equal(TEXT, "x")],
        ),
        select(at(0), vec![compare(TEXT, Test::Matches, "(")]),
    ]);
    let prepared = prepare(vec![erring]);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["y"], &[]),
        &[run_intent()],
        &[],
        &[],
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.12: a step over an invalid schema reports its error whatever its formula says.
fn a_schema_error_under_a_false_formula_is_reported() -> Seen {
    let mut runs = Vec::new();
    let invalid = plan(vec![
        select(
            PlanInput::Relation(Relation::Comment),
            vec![equal(TEXT, "y")],
        ),
        select(at(0), vec![equal(99, "z")]),
        step(Operation::Project {
            from: at(1),
            columns: vec![ColumnIndex(0)],
            distinct: RepeatedRows::Dropped,
        }),
    ]);
    let prepared = prepare(vec![invalid]);
    run_into(
        &mut runs,
        &prepared,
        &mut facts(&["x"], &[]),
        &[run_intent()],
        &[],
        &[],
    );
    Seen::Runs(Cow::Owned(runs))
}

/// R1.3 and R1.13: everything downstream of a bound observation may have rows in any file. A consumer of a producer's
/// named output is never absent, and a Replace over a Symbol column fed by it, where a name that left its file can
/// arrive, is unconditional even when its other conjunct cannot match.
fn consumers_of_a_bound_observation_are_never_absent() -> Seen {
    use ryo_execution_domain::domain::observation_schedule::types::{
        ObservationRequest, ResolvedObservationBinding, ResolvedObservationProducer,
    };
    use ryo_values::domain::observations::types::{
        BoundObservationInput, ObservationBindingName, ObservationColumn, ObservationColumnName,
        ObservationName, PlanObservation,
    };
    use ryo_values::domain::plan_parts::types::ObservationBindingId;
    use ryo_values::domain::records::types::{ObservationIndex, ObservationReadinessLayout};
    let columns = vec![
        ObservationColumn {
            name: ObservationColumnName("text".into()),
            kind: CellKind::Symbol,
        },
        ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        },
    ];
    let schema = BoundObservationInput {
        name: ObservationBindingName("named".into()),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    };
    let mut producer = plan(vec![step(Operation::Select {
        from: PlanInput::Relation(Relation::Comment),
        comparisons: vec![equal(TEXT, "y")],
        kept: vec![ColumnIndex(TEXT), ColumnIndex(2)],
    })]);
    producer.observations.push(PlanObservation {
        name: ObservationName("out".into()),
        source: at(0),
        columns,
        scope: StepScope::PerFile,
    });
    let mut consumer = plan(vec![
        select(PlanInput::BoundObservation(ObservationBindingId(0)), vec![]),
        step(Operation::Replace {
            from: at(0),
            text: ColumnIndex(0),
            pattern: LiteralText("a".into()),
            replacement: LiteralText("b".into()),
        }),
        select(
            PlanInput::Relation(Relation::Function),
            vec![equal(TEXT, "y")],
        ),
        step(Operation::Join {
            left: at(1),
            right: at(2),
            left_on: vec![ColumnIndex(1)],
            right_on: vec![ColumnIndex(2)],
            output: vec![JoinColumn {
                side: Side::Left,
                column: ColumnIndex(2),
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        }),
    ]);
    consumer.observation_inputs.push(schema.clone());
    let plans = vec![producer, consumer];
    let bindings = vec![
        vec![],
        vec![ResolvedObservationBinding {
            input: ObservationBindingId(0),
            schema,
            producers: vec![ResolvedObservationProducer::Output {
                rule: RuleIndex(0),
                output: ObservationIndex(0),
            }],
        }],
    ];
    let operators = crate::application::file_dag::service::prepare(
        &plans,
        &[vec![], vec![]],
        &[],
        Arc::new(ryo_execution_store::outbound::derived::factory::semantics()),
    )
    .expect("the observation plans prepare");
    let graph = ryo_execution_domain::domain::file_program::service::compile_prepared_observations(
        &plans,
        &operators,
        ObservationRequest::Bound {
            layout: ObservationReadinessLayout(Digest([7; 16])),
            bindings: &bindings,
        },
    );
    let presence = ryo_execution_domain::domain::executable_program::presence::service::of_graph(
        &graph,
        &|_| {
            ryo_execution_domain::domain::executable_program::presence::types::RelationScope::CurrentFile
        },
        ryo_execution_domain::domain::matching::types::PatternMatcher(
            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
        ),
        &ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers::default(
        ),
    );
    let consumer = graph.program_nodes[1]
        .iter()
        .map(|node| node.as_ref().expect("every consumer step has a node"))
        .collect::<Vec<_>>();
    Seen::Always(
        consumer
            .iter()
            .map(|node| {
                presence.nodes[node.0]
                    == ryo_execution_domain::domain::executable_program::presence::service::ALWAYS
            })
            .collect(),
    )
}
