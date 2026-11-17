//! A node reads its sources in the order its kernels read them. Every listed operator's instances read one input per role
//! they name, cover every row map and probe mode, and a probe's growth follows its mode. Each coverage check is shown failing
//! on a planted counter-example.
use super::value::Lowering;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::equality_join::service::mode_kind;
use crate::domain::equality_join::types::ProbeModeKind;
use crate::domain::growth::types::{Fan, Growth, WitnessColumns, Witnessed};
use crate::domain::physical_plan::service::lowering::service::row_map_kind;
use crate::domain::physical_plan::types::lowering::value::{
    BoundScan, DistinctNode, RowMapKind, ScanBoundNode,
};
use crate::domain::physical_plan::types::node_counters::value::NodeKind;
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::{ColumnIndex, HostedNumber};

/// What a case checks.
#[derive(Debug)]
enum Given {
    /// The rows this node reads.
    Reads(Lowering),
    /// Every instance names one role per input it reads.
    OneRolePerInput,
    /// The growth of each probe instance, by its mode.
    ProbeGrowth,
    /// The instances cover every row map kind and every probe mode.
    EveryKind,
    /// The same, with the instances of this probe mode left out.
    ModeLeftOut(ProbeModeKind),
    /// The same, with the instances of this row map kind left out.
    RowMapLeftOut(RowMapKind),
}

/// What a check answered.
#[derive(Debug, PartialEq)]
enum Answer {
    /// The rows a node reads, in order.
    Reads(std::borrow::Cow<'static, [RowSource]>),
    /// How many instances or kinds the check held for.
    Held(usize),
    /// Each probe instance's mode and growth, in order.
    Growths(std::borrow::Cow<'static, [(ProbeModeKind, Growth)]>),
}

/// What a check found broken.
#[derive(Debug, PartialEq)]
enum Breach {
    /// An instance of this kind names a role count other than its inputs'.
    RolesDiffer(NodeKind),
    /// No instance has this probe mode.
    ModeUnwitnessed(ProbeModeKind),
    /// No instance has this row map kind.
    RowMapUnwitnessed(RowMapKind),
}

type Input = Given;
type Value = Answer;
type Failure = Breach;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_reads_its_left_then_its_right"),
        word: Word::Succeeds,
        input: || {
            Given::Reads(Lowering::Distinct(DistinctNode {
                sources: [
                    RowSource::Node(PlanNodeId(0)),
                    RowSource::Node(PlanNodeId(1)),
                ],
            }))
        },
        expect: Expect::Value(Answer::Reads(std::borrow::Cow::Borrowed(&[
            RowSource::Node(PlanNodeId(0)),
            RowSource::Node(PlanNodeId(1)),
        ]))),
    },
    Case {
        name: CaseName("a_bound_scan_reads_nothing"),
        word: Word::Succeeds,
        input: || {
            Given::Reads(Lowering::ScanBound(ScanBoundNode {
                scan: BoundScan::Hosted(HostedNumber(0)),
            }))
        },
        expect: Expect::Value(Answer::Reads(std::borrow::Cow::Borrowed(&[]))),
    },
    Case {
        name: CaseName("every_instance_names_one_role_per_input"),
        word: Word::Succeeds,
        input: || Given::OneRolePerInput,
        expect: Expect::Value(Answer::Held(31)),
    },
    Case {
        name: CaseName("a_probe_grows_by_its_mode"),
        word: Word::Succeeds,
        input: || Given::ProbeGrowth,
        expect: Expect::Value(Answer::Growths(std::borrow::Cow::Borrowed(&[
            (ProbeModeKind::Inner, Growth::Keyed(Fan::Repeats)),
            (ProbeModeKind::Semi, Growth::Keyed(Fan::PerLeft)),
            (ProbeModeKind::Anti, Growth::Keyed(Fan::PerLeft)),
            (ProbeModeKind::Ordered, Growth::Keyed(Fan::PerLeft)),
        ]))),
    },
    Case {
        name: CaseName("the_instances_cover_every_row_map_and_probe_mode"),
        word: Word::Succeeds,
        input: || Given::EveryKind,
        expect: Expect::Value(Answer::Held(12)),
    },
    Case {
        name: CaseName("an_ordered_probe_left_out_fails_every_probe_mode"),
        word: Word::Refuses,
        input: || Given::ModeLeftOut(ProbeModeKind::Ordered),
        expect: Expect::Error(Breach::ModeUnwitnessed(ProbeModeKind::Ordered)),
    },
    Case {
        name: CaseName("a_words_map_left_out_fails_every_row_map"),
        word: Word::Refuses,
        input: || Given::RowMapLeftOut(RowMapKind::Words),
        expect: Expect::Error(Breach::RowMapUnwitnessed(RowMapKind::Words)),
    },
];

/// The generic columns the instances are built over.
const COLUMNS: WitnessColumns = WitnessColumns {
    key: ColumnIndex(0),
    order: ColumnIndex(1),
    id: ColumnIndex(2),
    parent: ColumnIndex(3),
    child: ColumnIndex(4),
    text: ColumnIndex(5),
    span: ColumnIndex(6),
};

/// The probe mode of an instance, when it is a probe.
fn mode(witness: &Witnessed<Lowering>) -> Option<ProbeModeKind> {
    if let Lowering::Probe(node) = &witness.node {
        Some(mode_kind(node.probe.mode))
    } else {
        None
    }
}

/// The row map kind of an instance, when it is a text map.
fn row_map(witness: &Witnessed<Lowering>) -> Option<RowMapKind> {
    if let Lowering::TextMap(node) = &witness.node {
        Some(row_map_kind(&node.map))
    } else {
        None
    }
}

/// Every probe mode and every row map kind is among `witnesses`.
fn every_kind(witnesses: &[(NodeKind, Witnessed<Lowering>)]) -> Result<Value, Failure> {
    for wanted in <ProbeModeKind as strum::VariantArray>::VARIANTS {
        if !witnesses
            .iter()
            .any(|(_, witness)| mode(witness) == Some(*wanted))
        {
            return Err(Breach::ModeUnwitnessed(*wanted));
        }
    }
    for wanted in <RowMapKind as strum::VariantArray>::VARIANTS {
        if !witnesses
            .iter()
            .any(|(_, witness)| row_map(witness) == Some(*wanted))
        {
            return Err(Breach::RowMapUnwitnessed(*wanted));
        }
    }
    Ok(Answer::Held(
        <ProbeModeKind as strum::VariantArray>::VARIANTS.len()
            + <RowMapKind as strum::VariantArray>::VARIANTS.len(),
    ))
}

fn subject(given: Input) -> Result<Value, Failure> {
    let witnesses = Lowering::witnesses(COLUMNS);
    match given {
        Given::Reads(lowering) => Ok(Answer::Reads(lowering.sources().collect())),
        Given::OneRolePerInput => {
            for (kind, witness) in &witnesses {
                if witness.node.sources().count() != witness.roles.len() {
                    return Err(Breach::RolesDiffer(*kind));
                }
            }
            Ok(Answer::Held(witnesses.len()))
        }
        Given::ProbeGrowth => Ok(Answer::Growths(
            witnesses
                .iter()
                .filter_map(|(_, witness)| mode(witness).map(|mode| (mode, witness.node.growth())))
                .collect(),
        )),
        Given::EveryKind => every_kind(&witnesses),
        Given::ModeLeftOut(left_out) => every_kind(
            &witnesses
                .into_iter()
                .filter(|(_, witness)| mode(witness) != Some(left_out))
                .collect::<Vec<_>>(),
        ),
        Given::RowMapLeftOut(left_out) => every_kind(
            &witnesses
                .into_iter()
                .filter(|(_, witness)| row_map(witness) != Some(left_out))
                .collect::<Vec<_>>(),
        ),
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
