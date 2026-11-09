//! One part of the `presence` service: the presence tests a select's comparisons, or a join's filter over its raw right
//! input, narrow the node to, grouped by the occurrence of the read each column comes from.
use super::types::{
    ColumnOrigin, ComparisonGroup, FormulaId, OriginComparison, PresenceFormula, PresenceTables, PresenceTest, TestBucket, TestId,
};
use ryo_relations::domain::relations::types::Relation;
use crate::domain::operator::contract::presence::InputMask;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, PlanComparison, QueryLiteral};
use ryo_values::domain::plans::types::Operation;

/// One test per occurrence of a read, AND-ed: each `Column <test> Literal(text)` comparison whose column has an origin,
/// renumbered to the relation's own column. Any other comparison only drops out, which weakens the test.
pub(super) fn narrowing<'c>(
    tables: &mut PresenceTables,
    comparisons: impl IntoIterator<Item = OriginComparison<'c>>,
) -> Vec<FormulaId> {
    let mut groups: Vec<ComparisonGroup> = Vec::new();
    for OriginComparison { comparison, origin } in comparisons {
        let (Operand::Column(_), Operand::Literal(QueryLiteral::Text(_)), Some(origin)) =
            (&comparison.left, &comparison.right, origin)
        else {
            continue;
        };
        let renumbered = PlanComparison {
            left: Operand::Column(origin.column),
            ..comparison.clone()
        };
        match groups
            .iter_mut()
            .find(|group| group.read == origin.read && group.occurrence == origin.occurrence)
        {
            Some(group) => group.comparisons.push(renumbered),
            None => groups.push(ComparisonGroup {
                read: origin.read,
                occurrence: origin.occurrence,
                comparisons: vec![renumbered],
            }),
        }
    }
    groups
        .into_iter()
        .filter_map(|group| {
            let relation = *tables.reads.get(group.read.0)?;
            Some(test(tables, relation, group.comparisons))
        })
        .collect()
}
fn pick(origins: &[Option<ColumnOrigin>], column: ColumnIndex) -> Option<ColumnOrigin> {
    origins.get(column.0 as usize).copied().flatten()
}
/// The narrowing tests of a select's comparisons, or of a join's filter over its raw right input. A reach join's filter
/// reads the closure's columns, which have no origin.
pub(super) fn node_narrowing(tables: &mut PresenceTables, operation: &Operation, origins: &[Vec<Option<ColumnOrigin>>]) -> Vec<FormulaId> {
    let (comparisons, input) = match operation {
        Operation::Select { comparisons, .. } => (comparisons, 0),
        Operation::Join {
            filter, reach: None, ..
        } => (filter, 1),
        _ => return Vec::new(),
    };
    let columns = origins.get(input).map_or(&[][..], Vec::as_slice);
    narrowing(
        tables,
        comparisons.iter().map(|comparison| {
            let origin = match comparison.left {
                Operand::Column(column) => pick(columns, column),
                _ => None,
            };
            OriginComparison { comparison, origin }
        }),
    )
}

/// The input whose rows a literal regular expression that does not compile fails on: the error is raised when a row
/// first reaches the comparison, and a consumer's presence formula may not read that input. A pattern read from the row
/// is declared by the operator's own failure mask.
pub(super) fn invalid_patterns(
    operation: &Operation,
    matcher: crate::domain::matching::types::PatternMatcher<'_>,
) -> Option<InputMask> {
    let (comparisons, mask) = match operation {
        Operation::Select { comparisons, .. } => (comparisons, InputMask(1)),
        Operation::Join { filter, .. } => (filter, InputMask(2)),
        Operation::Axis { domain_filter, .. } => (domain_filter, InputMask(4)),
        _ => return None,
    };
    comparisons
        .iter()
        .any(|comparison| match &comparison.right {
            Operand::Literal(QueryLiteral::Text(pattern))
                if comparison.test == ryo_values::domain::plan_parts::types::Test::Matches =>
            {
                crate::domain::matching::types::prepared::value::compile_regex(
                    crate::domain::table_texts::types::TextView(&pattern.0),
                    matcher,
                )
                .is_err()
            }
            _ => false,
        })
        .then_some(mask)
}

pub(super) fn formula(tables: &mut PresenceTables, formula: PresenceFormula) -> FormulaId {
    if let Some(found) = tables.formula_index.get(&formula) {
        return *found;
    }
    let id = FormulaId(tables.formulas.len());
    tables.formulas.push(formula.clone());
    let _new = tables.formula_index.insert(formula, id);
    id
}

pub(super) fn test(tables: &mut PresenceTables, relation: Relation, comparisons: Vec<PlanComparison>) -> FormulaId {
    use std::hash::{Hash, Hasher};
    let mut state = std::hash::DefaultHasher::new();
    relation.hash(&mut state);
    comparisons.hash(&mut state);
    let key = TestBucket(state.finish());
    let found = tables.test_buckets.get(&key).and_then(|bucket| {
        bucket.iter().copied().find(|id| {
            tables
                .tests
                .get(id.0)
                .is_some_and(|test| test.relation == relation && *test.comparisons == *comparisons)
        })
    });
    let id = found.unwrap_or_else(|| {
        let id = TestId(tables.tests.len());
        tables.tests.push(PresenceTest {
            relation,
            comparisons: comparisons.into_boxed_slice(),
            kinds: ryo_relations::domain::relations::service::kinds(relation).into(),
        });
        tables.test_buckets.entry(key).or_default().push(id);
        id
    });
    formula(tables, PresenceFormula::Test(id))
}
