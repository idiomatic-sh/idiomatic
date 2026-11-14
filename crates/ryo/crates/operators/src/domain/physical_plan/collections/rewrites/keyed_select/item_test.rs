//! The `keyed_select` item's cases, and the collection's shared cases when it has any: The `keyed_select` rewrite shares one index among two or more filters keyed on one name column, and leaves one alone.
use super::item::KeyedSelectItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem as _;
use crate::domain::physical_plan::types::{
    NodeReaders, PhysicalNode, Retention, RowSource,
};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
};

/// How many filters of `Defines` key on its name column.
type Input = usize;
/// How many indexes the rewrite shares, and how many nodes it turns into lookups.
type Value = (usize, usize);
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_keyed_filters_share_one_index"),
        word: Word::Succeeds,
        input: || 2,
        expect: Expect::Value((1, 2)),
    },
    Case {
        name: CaseName("one_keyed_filter_keeps_its_scan"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value((0, 0)),
    },
];

/// A filter of `Defines` whose name (column 3) equals `name`.
fn keyed(name: &str) -> PhysicalNode {
    PhysicalNode {
        lowering: Lowering::Filter {
            source: RowSource::Facts(Relation::Defines),
            select: crate::domain::select::service::lower(
                &[PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(3)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
                    origin: ComparisonOrigin::Unrecorded,
                }],
                &[],
                &[],
            ),
        },
        readers: NodeReaders(0),
        retained: Retention::Retained,
    }
}

/// Calls the part on one input; the author states it.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; a rewrite cannot fail"
)]
fn subject(filters: Input) -> Result<Value, Failure> {
    let mut nodes: Vec<PhysicalNode> = (0..filters).map(|at| keyed(&format!("name{at}"))).collect();
    let mut indexes = Vec::new();
    KeyedSelectItem.rewrite(&mut nodes, &mut indexes);
    let lookups = nodes
        .iter()
        .filter(|node| matches!(node.lowering, Lowering::Lookup { .. }))
        .count();
    Ok((indexes.len(), lookups))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
