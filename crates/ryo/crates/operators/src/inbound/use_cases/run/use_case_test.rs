//! The run use case through `plan` and `run` only: what a program answers, reads, holds and faults, against another run.
#[cfg(test)]
mod ordered_match;
#[cfg(test)]
mod render;
#[cfg(test)]
mod shared_index;
#[cfg(test)]
mod text_map;
use super::fixtures::cases::{
    DEFINES, TreeRun, defines, input_of, named, rewritten, select, self_matching, tree_probe_runs,
    walks_and_answers,
};
use super::fixtures::{TableInputs, TextResources, emitted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::ProbeMode;
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem;
use crate::domain::physical_plan::fixtures::ordered::{
    EDGES, ordered_existence_pair, ordered_rows,
};
use crate::domain::physical_plan::fixtures::{Step, case_program};
use crate::domain::physical_plan::service::REWRITES;
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::{Lowering, ProbeNode};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{KeyedIndex, PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, RepeatedRows};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::{RowCount, Table};

#[derive(Clone, Copy)]
enum Scenario {
    KeyedSelects,
    LookupAndFilter,
    EarlierRowAcrossStages,
    EarlierRowAcrossStagesInManyRows,
    DynamicRegexBeforeKey,
    PipelineHolds,
    ZeroColumns,
    RewritesKeepRows,
    RewriteChangingRows,
}
#[derive(Debug, PartialEq)]
enum Observation {
    /// How many times the facts of the relation were read.
    Reads(usize),
    /// Whether the two runs a case compares answered the same.
    Same(bool),
    /// How many node tables the run formed, and the most rows they held at once.
    Peak { formed: u32, peak_rows: usize },
    /// How many rows the emitted node holds.
    Count(RowCount),
}
type Input = Scenario;
type Value = Observation;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("selects_keyed_on_one_column_read_it_once"),
        word: Word::Succeeds,
        input: || Scenario::KeyedSelects,
        expect: Expect::Value(Observation::Reads(1)),
    },
    Case {
        name: CaseName("lookup_answers_what_filter_answers"),
        word: Word::Succeeds,
        input: || Scenario::LookupAndFilter,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("a_later_stage_fault_on_an_earlier_row_wins"),
        word: Word::Succeeds,
        input: || Scenario::EarlierRowAcrossStages,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("a_later_stage_fault_on_an_earlier_row_wins_across_chunks"),
        word: Word::Succeeds,
        input: || Scenario::EarlierRowAcrossStagesInManyRows,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("a_dynamic_regex_before_a_constant_key_keeps_its_error"),
        word: Word::Succeeds,
        input: || Scenario::DynamicRegexBeforeKey,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("facts_are_never_decoded"),
        word: Word::Succeeds,
        input: || Scenario::PipelineHolds,
        expect: Expect::Value(Observation::Peak {
            formed: 1,
            peak_rows: 1,
        }),
    },
    Case {
        name: CaseName("zero_column_rows_keep_their_count"),
        word: Word::Succeeds,
        input: || Scenario::ZeroColumns,
        expect: Expect::Value(Observation::Count(RowCount(4))),
    },
    Case {
        name: CaseName("every_rewrite_preserves_rows"),
        word: Word::Succeeds,
        input: || Scenario::RewritesKeepRows,
        expect: Expect::Value(Observation::Same(true)),
    },
    Case {
        name: CaseName("a_rewrite_changing_rows_fails_every_rewrite_preserves_rows"),
        word: Word::Succeeds,
        input: || Scenario::RewriteChangingRows,
        expect: Expect::Value(Observation::Same(false)),
    },
];

/// The planted counter-example of a rewrite: every anti probe made a semi probe, which keeps other rows.
struct AntiAsSemi;

impl RewritesItem for AntiAsSemi {
    fn rewrite(&self, nodes: &mut [PhysicalNode], _indexes: &mut Vec<KeyedIndex>) {
        for node in nodes {
            if let Lowering::Probe(ProbeNode { probe, .. }) = &mut node.lowering
                && probe.mode == ProbeMode::Anti
            {
                probe.mode = ProbeMode::Semi;
            }
        }
    }
}

/// Whether the probes of the nested tree answer the same rows, or the same first fault, planned with every rewrite of
/// `rewrites` and with each left out: inner probes and antijoins of the edges by parent and by child and of the elements by
/// id, over left rows of a node of three children, the root, that node again, a node of another file, a node past the tree,
/// and a token; an inner probe answering the same row for each child, one keyed on a column of no node, and one with a right
/// filter that cannot fail and drops every edge but a parent's first. Then rows narrower than their key against the edges
/// of the nested tree, which fault, and of a tree of only its root, which keep every row unread. Each run is planned with
/// every probe walking the tree and answers what it says, so no run passes by faulting alike or by never walking.
fn tree_probes_keep_rows(rewrites: &[&dyn RewritesItem]) -> Result<bool, Failure> {
    for TreeRun {
        steps,
        emits,
        inputs,
        mut resources,
        answer,
    } in tree_probe_runs()
    {
        let every = rewritten(&steps, &emits, rewrites, &inputs, &mut resources);
        if !walks_and_answers(&steps, &emits, rewrites, &every, &answer)? {
            return Ok(false);
        }
        for left_out in 0..rewrites.len() {
            let others: Vec<&dyn RewritesItem> = (0..rewrites.len())
                .filter(|at| *at != left_out)
                .filter_map(|at| rewrites.get(at).copied())
                .collect();
            if rewritten(&steps, &emits, &others, &inputs, &mut resources) != every {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Whether the case program, every node a rule's emit, and the two ordered existence shapes (the greatest and the least
/// ordinal per parent, ties at either end among them), only their antijoins emitted, answer the same rows planned with every
/// rewrite of `rewrites` and with each left out; and the probes of the tree as `tree_probes_keep_rows` runs them.
fn rewrites_keep_rows(rewrites: &[&dyn RewritesItem]) -> Result<Value, Failure> {
    let pairs = [
        ("kind", "alpha"),
        ("kind", "beta"),
        ("other", "alpha"),
        ("kind", "gamma"),
        ("other", "beta"),
    ];
    let mut resources = TextResources::new(&["kind", "other", "alpha", "beta", "gamma"]);
    let table = defines(&pairs, &resources);
    let edges = ordered_rows(resources.symbol("kind"));
    let inputs = TableInputs::new(&[(DEFINES, &table), (EDGES, &edges)]);
    let steps = case_program();
    let every_node: Vec<PlanNodeId> = (0..u32::try_from(steps.len()).unwrap_or(u32::MAX))
        .map(PlanNodeId)
        .collect();
    let programs = [
        (steps, every_node),
        (ordered_existence_pair(), vec![PlanNodeId(3), PlanNodeId(7)]),
    ];
    for (steps, emits) in &programs {
        let every = rewritten(steps, emits, rewrites, &inputs, &mut resources)?;
        for left_out in 0..rewrites.len() {
            let others: Vec<&dyn RewritesItem> = (0..rewrites.len())
                .filter(|at| *at != left_out)
                .filter_map(|at| rewrites.get(at).copied())
                .collect();
            if rewritten(steps, emits, &others, &inputs, &mut resources)? != every {
                return Ok(Observation::Same(false));
            }
        }
    }
    Ok(Observation::Same(tree_probes_keep_rows(rewrites)?))
}

/// The emitted rows of `steps`, every step a rule's emit when `all` is set, else only the last, over `pairs`.
fn answered(
    steps: &[Step],
    all: bool,
    pairs: &[(&str, &str)],
    texts: &[&str],
) -> (Result<Vec<Table>, Failure>, usize) {
    let mut resources = TextResources::new(texts);
    let table = defines(pairs, &resources);
    let inputs = TableInputs::new(&[(DEFINES, &table)]);
    let count = u32::try_from(steps.len()).unwrap_or(u32::MAX);
    let emits: Vec<PlanNodeId> = if all {
        (0..count).map(PlanNodeId).collect()
    } else {
        vec![PlanNodeId(count.saturating_sub(1))]
    };
    let answer = emitted(
        steps,
        &emits,
        &inputs,
        &mut resources,
        &mut FileBuffers::default(),
    );
    (answer, inputs.reads(DEFINES))
}

/// The texts every case's rows are made of.
const TEXTS: &[&str] = &["kind", "a", "[", "(", "n1", "n2", "absent"];

/// The fault of `steps`' last node over `pairs`, or the rows when it has none.
fn fault(steps: &[Step], pairs: &[(&str, &str)]) -> Result<Vec<Table>, Failure> {
    answered(steps, false, pairs, TEXTS).0
}

/// Stage 0 fails on row `late` (its refinement is no pattern), stage 1 on row `early` (its name is no pattern): the two-stage
/// pipeline reports stage 1's fault, the one stage 1 alone reports, and not stage 0's.
fn earlier_row_wins(rows: usize, early: usize, late: usize) -> Result<Value, Failure> {
    let pairs: Vec<(&str, &str)> = (0..rows)
        .map(|at| {
            (
                if at == late { "[" } else { "a" },
                if at == early { "(" } else { "a" },
            )
        })
        .collect();
    let stage0 = select(PlanInput::Relation(DEFINES), vec![self_matching(2)]);
    let stage1 = select(PlanInput::Node(PlanNodeId(0)), vec![self_matching(3)]);
    let alone1 = select(PlanInput::Relation(DEFINES), vec![self_matching(3)]);
    let both = fault(&[stage0.clone(), stage1], &pairs);
    let Err(first) = fault(&[alone1], &pairs) else {
        return Ok(Observation::Same(false));
    };
    let second = fault(&[stage0], &pairs);
    Ok(Observation::Same(
        both == Err(first.clone()) && second != Err(first),
    ))
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::KeyedSelects => {
            let names: Vec<String> = (0..43).map(|at| format!("n{at}")).collect();
            let texts: Vec<&str> = ["kind", "a"]
                .into_iter()
                .chain(names.iter().map(String::as_str))
                .collect();
            let pairs: Vec<(&str, &str)> = names.iter().map(|name| ("a", name.as_str())).collect();
            let steps: Vec<Step> = names
                .iter()
                .map(|name| select(PlanInput::Relation(DEFINES), vec![named(name)]))
                .collect();
            let (answer, reads) = answered(&steps, true, &pairs, &texts);
            answer.map(|_| Observation::Reads(reads))
        }
        Scenario::LookupAndFilter => {
            let pairs = [("a", "n2"), ("a", "n1"), ("a", "n2"), ("a", "n1")];
            let one = select(PlanInput::Relation(DEFINES), vec![named("n1")]);
            let absent = select(PlanInput::Relation(DEFINES), vec![named("absent")]);
            let keyed = answered(&[one.clone(), absent.clone()], true, &pairs, TEXTS).0?;
            let alone_one = answered(&[one], true, &pairs, TEXTS).0?;
            let alone_absent = answered(&[absent], true, &pairs, TEXTS).0?;
            Ok(Observation::Same(
                keyed.first() == alone_one.first() && keyed.get(1) == alone_absent.first(),
            ))
        }
        Scenario::EarlierRowAcrossStages => earlier_row_wins(3, 0, 2),
        Scenario::EarlierRowAcrossStagesInManyRows => earlier_row_wins(10_000, 4_100, 4_102),
        Scenario::DynamicRegexBeforeKey => {
            let pairs = [("[", "n1"), ("a", "n1")];
            let guarded = select(
                PlanInput::Relation(DEFINES),
                vec![self_matching(2), named("n1")],
            );
            let keyed = answered(&[guarded.clone(), guarded.clone()], true, &pairs, TEXTS).0;
            let Err(alone) = answered(&[guarded], true, &pairs, TEXTS).0 else {
                return Ok(Observation::Same(false));
            };
            // What a select answers when nothing of it ran: its relation is not held. The regex's fault is another.
            let unheld = select(PlanInput::Relation(Relation::Comment), Vec::new());
            let unrun = answered(&[unheld], true, &pairs, TEXTS).0;
            Ok(Observation::Same(
                keyed == Err(alone.clone()) && unrun != Err(alone),
            ))
        }
        Scenario::PipelineHolds => {
            let mut resources = TextResources::new(TEXTS);
            let table = defines(&[("a", "n1"), ("a", "n2"), ("a", "n1")], &resources);
            let inputs = TableInputs::new(&[(DEFINES, &table)]);
            let steps = [
                select(PlanInput::Relation(DEFINES), vec![named("n1")]),
                Step {
                    operation: Operation::Project {
                        from: input_of(PlanInput::Node(PlanNodeId(0))),
                        columns: vec![ColumnIndex(1)],
                        distinct: RepeatedRows::Dropped,
                    },
                    inputs: vec![PlanInput::Node(PlanNodeId(0))],
                },
            ];
            let mut buffers = FileBuffers::default();
            let _rows = emitted(
                &steps,
                &[PlanNodeId(1)],
                &inputs,
                &mut resources,
                &mut buffers,
            )?;
            Ok(Observation::Peak {
                formed: buffers.meter.formed.0,
                peak_rows: buffers.meter.peak_rows.0,
            })
        }
        Scenario::RewritesKeepRows => rewrites_keep_rows(REWRITES),
        Scenario::RewriteChangingRows => {
            let mut planted: Vec<&dyn RewritesItem> = REWRITES.to_vec();
            planted.push(&AntiAsSemi);
            rewrites_keep_rows(&planted)
        }
        Scenario::ZeroColumns => {
            let mut resources = TextResources::new(TEXTS);
            let table = defines(
                &[("a", "n1"), ("a", "n1"), ("a", "n2"), ("a", "n1")],
                &resources,
            );
            let inputs = TableInputs::new(&[(DEFINES, &table)]);
            let steps = [Step {
                operation: Operation::Project {
                    from: StepInput::Relation(DEFINES),
                    columns: Vec::new(),
                    distinct: RepeatedRows::Kept,
                },
                inputs: vec![PlanInput::Relation(DEFINES)],
            }];
            let rows = emitted(
                &steps,
                &[PlanNodeId(0)],
                &inputs,
                &mut resources,
                &mut FileBuffers::default(),
            )?;
            Ok(Observation::Count(
                rows.first().map_or(RowCount(0), |table| table.count),
            ))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod axis;
#[cfg(test)]
mod axis_layout;
#[cfg(test)]
mod closure;
#[cfg(test)]
mod cross_kind;
#[cfg(test)]
mod fed;
#[cfg(test)]
mod graphs;
#[cfg(test)]
mod held_rows;
#[cfg(test)]
mod kinds;
#[cfg(test)]
mod relational;
#[cfg(test)]
mod streaming;
#[cfg(test)]
mod tree_rows;
