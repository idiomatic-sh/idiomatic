//! Named captures keep their identity; fused and held programs agree on rows, order and faults.
use super::super::fixtures::routes::{Combine, Feed, Fusion};
use super::super::fixtures::semantics::{Calls, MeaningSnapshot};
use super::fixtures::{Capture, Decision, capture, decide};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::executable_program::fusion::types::{
    Barrier, Fusibility, Ineligible,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::borrow::Cow;

/// What a row compiles and runs: the named continuation program; one combining operation over
/// every pair of feeds, fused and held; or one pair of feeds, fused and held.
#[derive(Clone, Copy, Debug)]
enum Fusing {
    NamedCapture,
    EveryFeed(Combine),
    Pair {
        combine: Combine,
        left: Feed,
        right: Feed,
    },
}

/// What the program's classes, or the fused and held runs, answered.
#[derive(Debug, PartialEq)]
enum Fused {
    Capture(Capture),
    /// For each pair of feeds (left, then right, each in `Feed::ALL` order), whether the fused and
    /// held runs answered alike and reached the same faults; and the classes each fusion gave.
    Agreement {
        agreeing: Cow<'static, [bool]>,
        fused_classes: Cow<'static, [Option<Fusibility>]>,
        held_classes: Cow<'static, [Option<Fusibility>]>,
    },
    Pair {
        fused: Decision,
        held: Decision,
    },
}

type Input = Fusing;
type Value = Fused;
type Failure = OperatingError;

const FUSED: Option<Fusibility> = Some(Fusibility::Fusable);
const HELD: Option<Fusibility> = Some(Fusibility::Barrier(Barrier::SharedConsumer));

/// Every pair of feeds agreeing, each fusion giving its one class.
const fn agreement() -> Fused {
    Fused::Agreement {
        agreeing: Cow::Borrowed(&[true; 9]),
        fused_classes: Cow::Borrowed(&[FUSED]),
        held_classes: Cow::Borrowed(&[HELD]),
    }
}

/// A fused and a held run that both answered this and reached these failed, appended and union
/// calls.
const fn alike(answer: &'static str, failed: usize, appended: usize, unions: usize) -> Fused {
    let reached = MeaningSnapshot {
        failed: Calls(failed),
        appended: Calls(appended),
        projects: Calls(0),
        unions: Calls(unions),
    };
    Fused::Pair {
        fused: Decision {
            answer: Cow::Borrowed(answer),
            reached,
            classes: [FUSED; 2],
            readiness: false,
        },
        held: Decision {
            answer: Cow::Borrowed(answer),
            reached,
            classes: [HELD; 2],
            readiness: false,
        },
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_named_capture_root_is_not_fused_into"),
        word: Word::Succeeds,
        input: || Fusing::NamedCapture,
        expect: Expect::Value(Fused::Capture(Capture {
            projections: [(Some(Fusibility::Barrier(Barrier::Capture)), 1); 2],
            consumer: Some(Fusibility::Excluded(Ineligible::Source)),
        })),
    },
    Case {
        name: CaseName("fused_and_held_combinations_agree_on_rows_order_and_error_precedence"),
        word: Word::Succeeds,
        input: || Fusing::EveryFeed(Combine::Join),
        expect: Expect::Value(agreement()),
    },
    Case {
        name: CaseName("fused_and_held_unions_agree_on_every_feed"),
        word: Word::Succeeds,
        input: || Fusing::EveryFeed(Combine::Union),
        expect: Expect::Value(agreement()),
    },
    Case {
        name: CaseName("fused_and_held_intersections_agree_on_every_feed"),
        word: Word::Succeeds,
        input: || Fusing::EveryFeed(Combine::Intersection),
        expect: Expect::Value(agreement()),
    },
    Case {
        name: CaseName("fused_and_held_differences_agree_on_every_feed"),
        word: Word::Succeeds,
        input: || Fusing::EveryFeed(Combine::Difference),
        expect: Expect::Value(agreement()),
    },
    Case {
        name: CaseName("fused_and_held_antijoins_agree_on_every_feed"),
        word: Word::Succeeds,
        input: || Fusing::EveryFeed(Combine::Antijoin),
        expect: Expect::Value(agreement()),
    },
    Case {
        name: CaseName("an_empty_left_does_not_hide_a_failing_right_differently_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Join,
            left: Feed::Empty,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("an_empty_left_union_hides_a_failing_right_alike_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Union,
            left: Feed::Empty,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("an_empty_left_intersection_hides_a_failing_right_alike_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Intersection,
            left: Feed::Empty,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("an_empty_left_difference_hides_a_failing_right_alike_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Difference,
            left: Feed::Empty,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("an_empty_left_antijoin_hides_a_failing_right_alike_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Antijoin,
            left: Feed::Empty,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("competing_faults_reach_the_same_side_first_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Join,
            left: Feed::Failing,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("competing_union_faults_reach_the_same_side_first_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Union,
            left: Feed::Failing,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("competing_intersection_faults_reach_the_same_side_first_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Intersection,
            left: Feed::Failing,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("competing_difference_faults_reach_the_same_side_first_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Difference,
            left: Feed::Failing,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
    Case {
        name: CaseName("competing_antijoin_faults_reach_the_same_side_first_when_fused"),
        word: Word::Succeeds,
        input: || Fusing::Pair {
            combine: Combine::Antijoin,
            left: Feed::Failing,
            right: Feed::Failing,
        },
        expect: Expect::Value(alike("failed WrongOperation", 1, 0, 0)),
    },
];

/// Adds each class the decision gave that `classes` does not have yet.
fn gather(classes: &mut Vec<Option<Fusibility>>, decision: &Decision) {
    for class in decision.classes {
        if !classes.contains(&class) {
            classes.push(class);
        }
    }
}

/// Compiles and classes the named program, or runs the combining program fused and held.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Fusing::NamedCapture => capture().map(Fused::Capture),
        Fusing::EveryFeed(combine) => {
            let mut agreeing = Vec::new();
            let mut fused_classes = Vec::new();
            let mut held_classes = Vec::new();
            for (left, right) in Feed::ALL
                .into_iter()
                .flat_map(|left| Feed::ALL.map(|right| (left, right)))
            {
                let fused = decide(combine, left, right, Fusion::Fused)?;
                let held = decide(combine, left, right, Fusion::Held)?;
                agreeing.push(fused.agrees(&held));
                gather(&mut fused_classes, &fused);
                gather(&mut held_classes, &held);
            }
            Ok(Fused::Agreement {
                agreeing: Cow::Owned(agreeing),
                fused_classes: Cow::Owned(fused_classes),
                held_classes: Cow::Owned(held_classes),
            })
        }
        Fusing::Pair {
            combine,
            left,
            right,
        } => Ok(Fused::Pair {
            fused: decide(combine, left, right, Fusion::Fused)?,
            held: decide(combine, left, right, Fusion::Held)?,
        }),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
