//! Declaration identity is typed and length-delimited, and readiness is bound to its layout and never defaults to complete.
use super::errors::ObservationReadinessError;
use super::service::{
    frontier_state, identity, layout_identity, new_frontiers, set_frontier, validate_frontiers,
};
use super::types::*;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::records::types::{FrontierReadiness, FrontierRefusal, FrontierRefusalReason};
use crate::domain::runs::types::RuleId;
use crate::domain::stored_facts::types::Digest;
use ryo_relations::domain::relations::types::CellKind;

#[derive(Clone, Copy)]
enum Scenario {
    NoDeclarations,
    TabAcrossBoundary,
    ColumnKind,
    UnavailableProducer,
    SameDeclaration,
    CompleteSlot,
    UnknownSlot,
    MissingSlot,
    RuleOrder,
    ProducerConfiguration,
    DroppedRule,
    ReorderedLayout,
    ShorterCount,
    OldVersion,
    InvalidCompleteBits,
    CompleteAndSkipped,
    MissingBits,
    DuplicateRefusals,
}
type Input = Scenario;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Empty,
    Distinct,
    Same,
    Complete,
    Unknown,
}
type Value = Answer;
type Failure = ObservationReadinessError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_declarations_keep_the_ordinary_identity_empty"),
        word: Word::Succeeds,
        input: || Scenario::NoDeclarations,
        expect: Expect::Value(Answer::Empty),
    },
    Case {
        name: CaseName("a_tab_moved_across_the_rule_and_output_boundary_changes_identity"),
        word: Word::Succeeds,
        input: || Scenario::TabAcrossBoundary,
        expect: Expect::Value(Answer::Distinct),
    },
    Case {
        name: CaseName("a_column_kind_changes_identity"),
        word: Word::Succeeds,
        input: || Scenario::ColumnKind,
        expect: Expect::Value(Answer::Distinct),
    },
    Case {
        name: CaseName("an_unavailable_producer_changes_identity"),
        word: Word::Succeeds,
        input: || Scenario::UnavailableProducer,
        expect: Expect::Value(Answer::Distinct),
    },
    Case {
        name: CaseName("the_same_declaration_keeps_its_identity"),
        word: Word::Succeeds,
        input: || Scenario::SameDeclaration,
        expect: Expect::Value(Answer::Same),
    },
    Case {
        name: CaseName("a_published_slot_is_complete"),
        word: Word::Succeeds,
        input: || Scenario::CompleteSlot,
        expect: Expect::Value(Answer::Complete),
    },
    Case {
        name: CaseName("a_refused_slot_is_unknown"),
        word: Word::Succeeds,
        input: || Scenario::UnknownSlot,
        expect: Expect::Value(Answer::Unknown),
    },
    Case {
        name: CaseName("a_missing_publication_is_not_an_empty_complete_output"),
        word: Word::Succeeds,
        input: || Scenario::MissingSlot,
        expect: Expect::Value(Answer::Unknown),
    },
    Case {
        name: CaseName("the_order_of_selected_rules_defines_the_layout"),
        word: Word::Succeeds,
        input: || Scenario::RuleOrder,
        expect: Expect::Value(Answer::Distinct),
    },
    Case {
        name: CaseName("producer_configuration_defines_the_layout"),
        word: Word::Succeeds,
        input: || Scenario::ProducerConfiguration,
        expect: Expect::Value(Answer::Distinct),
    },
    Case {
        name: CaseName("a_dropped_rule_defines_another_layout"),
        word: Word::Succeeds,
        input: || Scenario::DroppedRule,
        expect: Expect::Value(Answer::Distinct),
    },
    Case {
        name: CaseName("a_reordered_layout_cannot_reinterpret_completion_bits"),
        word: Word::Refuses,
        input: || Scenario::ReorderedLayout,
        expect: Expect::Error(ObservationReadinessError::Invalid),
    },
    Case {
        name: CaseName("a_different_slot_count_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ShorterCount,
        expect: Expect::Error(ObservationReadinessError::Invalid),
    },
    Case {
        name: CaseName("an_old_readiness_version_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OldVersion,
        expect: Expect::Error(ObservationReadinessError::Invalid),
    },
    Case {
        name: CaseName("completion_bits_beyond_the_slots_are_refused"),
        word: Word::Refuses,
        input: || Scenario::InvalidCompleteBits,
        expect: Expect::Error(ObservationReadinessError::Invalid),
    },
    Case {
        name: CaseName("a_slot_both_complete_and_skipped_is_refused"),
        word: Word::Refuses,
        input: || Scenario::CompleteAndSkipped,
        expect: Expect::Error(ObservationReadinessError::Invalid),
    },
    Case {
        name: CaseName("missing_completion_words_are_refused"),
        word: Word::Refuses,
        input: || Scenario::MissingBits,
        expect: Expect::Error(ObservationReadinessError::Invalid),
    },
    Case {
        name: CaseName("repeated_refusals_for_one_slot_are_refused"),
        word: Word::Refuses,
        input: || Scenario::DuplicateRefusals,
        expect: Expect::Error(ObservationReadinessError::Invalid),
    },
];

fn declaration(name: &str, rule: &str, output: &str) -> ObservationBinding {
    ObservationBinding {
        name: ObservationBindingName(name.into()),
        columns: vec![ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        }],
        producers: vec![ObservationProducer::Output {
            rule: RuleId(rule.into()),
            output: ObservationName(output.into()),
        }],
    }
}

fn rule(name: &str, configuration: u8) -> ObservationRuleLayout {
    ObservationRuleLayout {
        rule: RuleId(name.into()),
        frontiers: ObservationFrontierCount(1),
        outputs: ObservationOutputLayout::Persistent {
            configuration: Digest([configuration; 16]),
            outputs: Vec::new(),
        },
    }
}

fn verdict<T: PartialEq>(left: T, right: T) -> Answer {
    if left == right {
        Answer::Same
    } else {
        Answer::Distinct
    }
}

fn readiness(state: ProducerReadiness) -> Answer {
    match state {
        ProducerReadiness::Complete => Answer::Complete,
        ProducerReadiness::Unknown(_) | ProducerReadiness::Skipped | ProducerReadiness::NoQuery => {
            Answer::Unknown
        }
    }
}

fn published() -> (
    FrontierReadiness,
    ObservationReadinessLayout,
    ObservationReadinessCount,
) {
    let layout = ObservationReadinessLayout(Digest([3; 16]));
    let count = ObservationReadinessCount(4);
    let mut map = new_frontiers(layout, count);
    for (slot, state) in [
        (0, ProducerReadiness::Complete),
        (
            1,
            ProducerReadiness::Unknown(ObservationReason("reader refused\tλ\0payload".into())),
        ),
        (2, ProducerReadiness::Skipped),
    ] {
        set_frontier(&mut map, ObservationReadinessSlot(slot), state)
            .expect("the slot is inside the map");
    }
    (map, layout, count)
}

fn blank() -> (
    FrontierReadiness,
    ObservationReadinessLayout,
    ObservationReadinessCount,
) {
    let layout = ObservationReadinessLayout(Digest([9; 16]));
    let count = ObservationReadinessCount(1);
    (new_frontiers(layout, count), layout, count)
}

fn state_of(slot: u32) -> Result<Value, Failure> {
    let (map, layout, count) = published();
    let checked = validate_frontiers(&map, layout, count)?;
    frontier_state(checked, ObservationReadinessSlot(slot)).map(readiness)
}

fn refused_by(
    changed: &FrontierReadiness,
    layout: ObservationReadinessLayout,
    count: ObservationReadinessCount,
) -> Result<Value, Failure> {
    validate_frontiers(changed, layout, count).map(|_| Answer::Same)
}

fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::NoDeclarations => Ok(if identity(&[]).0.is_empty() {
            Answer::Empty
        } else {
            Answer::Same
        }),
        Scenario::TabAcrossBoundary => Ok(verdict(identity(&[declaration("input", "a\tb", "c")]), identity(&[declaration("input", "a", "b\tc")]))),
        Scenario::ColumnKind => {
            let left = declaration("input", "a\tb", "c");
            let mut typed = left.clone();
            typed.columns[0].kind = CellKind::Number;
            Ok(verdict(identity(&[left]), identity(&[typed])))
        }
        Scenario::UnavailableProducer => {
            let left = declaration("input", "a\tb", "c");
            let mut unavailable = left.clone();
            unavailable.producers = vec![ObservationProducer::NoQuery {
                rule: RuleId("a\tb".into()),
            }];
            Ok(verdict(identity(&[left]), identity(&[unavailable])))
        }
        Scenario::SameDeclaration => {
            let left = declaration("input", "a\tb", "c");
            Ok(verdict(identity(&[left.clone()]), identity(&[left])))
        }
        Scenario::CompleteSlot => state_of(0),
        Scenario::UnknownSlot => state_of(1),
        Scenario::MissingSlot => state_of(3),
        Scenario::RuleOrder => {
            let (first, second) = (rule("producer-a", 1), rule("producer-b", 2));
            Ok(verdict(layout_identity(&[first.clone(), second.clone()]), layout_identity(&[second, first])))
        }
        Scenario::ProducerConfiguration => {
            let first = rule("producer-a", 1);
            Ok(verdict(layout_identity(&[first.clone(), rule("producer-b", 2)]), layout_identity(&[first, rule("producer-b", 3)])))
        }
        Scenario::DroppedRule => {
            let (first, second) = (rule("producer-a", 1), rule("producer-b", 2));
            Ok(verdict(
                layout_identity(&[first.clone(), second]),
                layout_identity(&[first]),
            ))
        }
        Scenario::ReorderedLayout => {
            let (map, _, count) = published();
            refused_by(&map, ObservationReadinessLayout(Digest([4; 16])), count)
        }
        Scenario::ShorterCount => {
            let (map, layout, _) = published();
            refused_by(&map, layout, ObservationReadinessCount(3))
        }
        Scenario::OldVersion => {
            let (mut map, layout, count) = published();
            map.version = ObservationReadinessVersion(1);
            refused_by(&map, layout, count)
        }
        Scenario::InvalidCompleteBits => {
            let (mut map, layout, count) = blank();
            map.complete[0] = 2;
            refused_by(&map, layout, count)
        }
        Scenario::CompleteAndSkipped => {
            let (mut map, layout, count) = blank();
            map.complete[0] = 1;
            map.skipped = vec![1];
            refused_by(&map, layout, count)
        }
        Scenario::MissingBits => {
            let (mut map, layout, count) = blank();
            map.complete.clear();
            refused_by(&map, layout, count)
        }
        Scenario::DuplicateRefusals => {
            let (mut map, layout, count) = blank();
            map.refusals = vec![
                FrontierRefusal {
                    slot: ObservationReadinessSlot(0),
                    reason: FrontierRefusalReason::NoQuery
                };
                2
            ];
            refused_by(&map, layout, count)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
