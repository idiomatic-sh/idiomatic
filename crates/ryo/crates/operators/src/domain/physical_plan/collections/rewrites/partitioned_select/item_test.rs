//! The `partitioned_select` item's cases: sibling filters keyed on one column by different constants share one partition, a
//! constant asked twice shares one slot, and a filter alone keeps its scan.
use super::item::partitions;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::{NodeReaders, RowSource};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};

/// The constants of the tree filters keyed on its text column (2).
type Input = &'static [&'static str];
/// How many partitions, how many slots the first has, and how many nodes read one.
type Value = (usize, usize, usize);
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("sibling_filters_share_one_partition"),
        word: Word::Succeeds,
        input: || &["for", "while", "with"],
        expect: Expect::Value((1, 3, 3)),
    },
    Case {
        name: CaseName("one_constant_asked_twice_shares_a_slot"),
        word: Word::Succeeds,
        input: || &["for", "for"],
        expect: Expect::Value((1, 1, 2)),
    },
    Case {
        name: CaseName("one_filter_keeps_its_scan"),
        word: Word::Succeeds,
        input: || &["for"],
        expect: Expect::Value((0, 0, 0)),
    },
];

/// A filter of the tree whose text (column 2) equals `text`.
fn keyed(text: &str) -> PhysicalNode {
    PhysicalNode {
        lowering: Lowering::Filter {
            source: RowSource::Tree(Relation::SyntaxTree),
            select: crate::domain::select::service::lower(
                &[PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(2)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                &[],
                &[],
            ),
        },
        readers: NodeReaders(0),
        engine: crate::domain::physical_plan::types::EngineUses(1),
        flow: crate::domain::physical_plan::types::NodeFlow::Held,
    }
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; the analysis cannot fail"
)]
fn subject(texts: Input) -> Result<Value, Failure> {
    let nodes: Vec<PhysicalNode> = texts.iter().map(|text| keyed(text)).collect();
    let found = partitions(&nodes);
    Ok((
        found.groups.len(),
        found.groups.first().map_or(0, |group| group.keys.len()),
        found.of_node.iter().flatten().count(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
