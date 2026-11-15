//! The `tree_probe` item's cases: the probes whose index the open file's tree is, and the ones that keep a hash index.
use super::item::TreeProbeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::equality_join::types::{JoinShape, OrderedProbe, PartitionExtremum, ProbeMode};
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem as _;
use crate::domain::physical_plan::service::indexes::service::shared_indexes;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{
    NodeReaders, PlanNodeId, ProbeIndex, Retention, RowSource,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, JoinColumn, LiteralText, Operand, PlanComparison, QueryLiteral,
    Side, Test,
};
use std::borrow::Cow;

/// The probes planned.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// The edges by their parent.
    ChildByParent,
    /// The edges by their child.
    ChildByChild,
    /// The elements by their id.
    TreeById,
    /// The elements by their span.
    TreeBySpan,
    /// The elements by their file and span.
    TreeByFileAndSpan,
    /// The edges by their parent, ordered by their ordinal.
    OrderedByParent,
    /// The edges by their parent, the right rows filtered by a pattern, which can fail.
    FallibleFilter,
    /// A relation of the facts by its node column.
    Facts,
    /// Two probes of the edges by their parent.
    TwoByParent,
}
type Input = Shape;
/// The index of each probe after the rewrite and the sharing step, and how many indexes the plan shares.
type Value = (Cow<'static, [ProbeIndex]>, usize);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("syntax_child_by_parent_reads_the_tree"),
        word: Word::Succeeds,
        input: || Shape::ChildByParent,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Tree]), 0)),
    },
    Case {
        name: CaseName("syntax_child_by_child_reads_the_tree"),
        word: Word::Succeeds,
        input: || Shape::ChildByChild,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Tree]), 0)),
    },
    Case {
        name: CaseName("syntax_tree_by_id_reads_the_tree"),
        word: Word::Succeeds,
        input: || Shape::TreeById,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Tree]), 0)),
    },
    Case {
        name: CaseName("a_span_key_keeps_its_index"),
        word: Word::Succeeds,
        input: || Shape::TreeBySpan,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Own]), 0)),
    },
    Case {
        name: CaseName("two_key_columns_keep_their_index"),
        word: Word::Succeeds,
        input: || Shape::TreeByFileAndSpan,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Own]), 0)),
    },
    Case {
        name: CaseName("an_ordered_probe_keeps_its_index"),
        word: Word::Succeeds,
        input: || Shape::OrderedByParent,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Own]), 0)),
    },
    Case {
        name: CaseName("a_fallible_right_filter_keeps_its_index"),
        word: Word::Succeeds,
        input: || Shape::FallibleFilter,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Own]), 0)),
    },
    Case {
        name: CaseName("a_facts_right_keeps_its_index"),
        word: Word::Succeeds,
        input: || Shape::Facts,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Own]), 0)),
    },
    Case {
        name: CaseName("two_tree_probes_share_nothing"),
        word: Word::Succeeds,
        input: || Shape::TwoByParent,
        expect: Expect::Value((Cow::Borrowed(&[ProbeIndex::Tree, ProbeIndex::Tree]), 0)),
    },
];

/// The join of node 0's first column with `relation`'s rows keyed on `right_on`, answering the right's first column, its
/// right rows kept by `filter`.
fn joined(relation: RowSource, right_on: &[u32], filter: &[PlanComparison]) -> PhysicalNode {
    let left_on: Vec<ColumnIndex> = right_on.iter().map(|_| ColumnIndex(0)).collect();
    let right_on: Vec<ColumnIndex> = right_on.iter().copied().map(ColumnIndex).collect();
    let output = [JoinColumn {
        side: Side::Right,
        column: ColumnIndex(0),
    }];
    PhysicalNode {
        lowering: Lowering::Probe {
            left: RowSource::Node(PlanNodeId(0)),
            right: relation,
            probe: crate::domain::equality_join::service::lower(
                JoinShape {
                    left_on: &left_on,
                    right_on: &right_on,
                    output: &output,
                    view: &[],
                    reach: None,
                },
                filter,
                &[],
            ),
            index: ProbeIndex::Own,
        },
        readers: NodeReaders(0),
        retained: Retention::Retained,
        flow: crate::domain::physical_plan::types::Flow::Held,
    }
}

fn subject(shape: Input) -> Result<Value, Failure> {
    let edges = RowSource::Tree(Relation::SyntaxChild);
    let tree = RowSource::Tree(Relation::SyntaxTree);
    let mut nodes = match shape {
        Shape::ChildByParent => vec![joined(edges, &[0], &[])],
        Shape::ChildByChild => vec![joined(edges, &[1], &[])],
        Shape::TreeById => vec![joined(tree, &[0], &[])],
        Shape::TreeBySpan => vec![joined(tree, &[4], &[])],
        Shape::TreeByFileAndSpan => vec![joined(tree, &[3, 4], &[])],
        Shape::OrderedByParent => {
            let mut node = joined(edges, &[0], &[]);
            if let Lowering::Probe { probe, .. } = &mut node.lowering {
                probe.mode = ProbeMode::Ordered(OrderedProbe {
                    left_order: ColumnIndex(2),
                    right_order: ColumnIndex(2),
                    extremum: PartitionExtremum::Maximum,
                });
            }
            vec![node]
        }
        Shape::FallibleFilter => vec![joined(
            edges,
            &[0],
            &[PlanComparison {
                test: Test::Matches,
                left: Operand::Column(ColumnIndex(3)),
                right: Operand::Literal(QueryLiteral::Text(LiteralText("[".into()))),
                origin: ComparisonOrigin::Unrecorded,
            }],
        )],
        Shape::Facts => vec![joined(RowSource::Facts(Relation::Defines), &[0], &[])],
        Shape::TwoByParent => vec![joined(edges, &[0], &[]), joined(edges, &[0], &[])],
    };
    let mut indexes = Vec::new();
    TreeProbeItem.rewrite(&mut nodes, &mut indexes);
    shared_indexes(&mut nodes, &mut indexes);
    let index_of = |node: &PhysicalNode| {
        if let Lowering::Probe { index, .. } = &node.lowering {
            Some(*index)
        } else {
            None
        }
    };
    Ok((
        Cow::Owned(nodes.iter().filter_map(index_of).collect()),
        indexes.len(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
