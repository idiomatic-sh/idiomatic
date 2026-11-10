//! One part of the `presence` service: the presence tests a select's comparisons, or a join's filter over its raw right
//! input, narrow the node to, grouped by the occurrence of the read each column comes from; the literal patterns that make
//! an input fallible; and which span columns are located, holding only spans a file's provider admitted or matches inside
//! them. Locatedness is a fact of a column's values, not of the read they came from, so unlike an origin it survives a
//! union whose two sides are both located.
use super::types::{
    ColumnOrigin, ComparisonGroup, FormulaId, LocatedId, OriginComparison, PresenceFormula,
    PresenceTables, PresenceTest, SpanColumn, TestBucket, TestId,
};
use crate::domain::matching::types::PatternMatcher;
use crate::domain::matching::types::prepared::value::PreparedMatchers;
use crate::domain::operation_schema::projection::output_origins;
use crate::domain::operation_schema::types::InputShape;
use crate::domain::operator::contract::presence::InputMask;
use crate::domain::table_texts::types::TextView;
use ryo_relations::domain::relations::types::{CellKind, Relation};
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
pub(super) fn node_narrowing(
    tables: &mut PresenceTables,
    operation: &Operation,
    origins: &[Vec<Option<ColumnOrigin>>],
) -> Vec<FormulaId> {
    let (comparisons, input) = match operation {
        Operation::Select { comparisons, .. } => (comparisons, 0),
        Operation::Join {
            filter,
            reach: None,
            ..
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
/// first reaches the comparison or the replacement, and a consumer's presence formula may not read that input. A pattern
/// read from the row is declared by the operator's own failure mask. The match names every operation, so a new one that
/// carries a literal pattern cannot be left out. A source search's pattern is compiled into its prepared slot, which the
/// run then reads, and only a pattern the request did not declare is compiled here and dropped.
pub(super) fn invalid_patterns(
    operation: &Operation,
    matcher: PatternMatcher<'_>,
    prepared: &PreparedMatchers,
) -> Option<InputMask> {
    let invalid = |pattern: &ryo_values::domain::plan_parts::types::LiteralText| {
        crate::domain::matching::types::prepared::value::compile_regex(
            TextView(&pattern.0),
            matcher,
        )
        .is_err()
    };
    let (comparisons, mask) = match operation {
        Operation::Select { comparisons, .. } => (comparisons, InputMask(1)),
        Operation::Join { filter, .. } => (filter, InputMask(2)),
        Operation::Axis { domain_filter, .. } => (domain_filter, InputMask(4)),
        Operation::SourceMatches { pattern, .. } => {
            let refused = prepared
                .regex(TextView(&pattern.0), matcher)
                .map_or_else(|| invalid(pattern), |compiled| compiled.is_err());
            return refused.then_some(InputMask(1));
        }
        Operation::Replace { pattern, .. } => {
            return invalid(pattern).then_some(InputMask(1));
        }
        Operation::Project { .. }
        | Operation::Extend { .. }
        | Operation::Union { .. }
        | Operation::Intersection { .. }
        | Operation::Difference { .. }
        | Operation::Antijoin { .. }
        | Operation::GroupCount { .. }
        | Operation::GroupDigest { .. }
        | Operation::Capture { .. }
        | Operation::Rank { .. }
        | Operation::OrderedMatch { .. }
        | Operation::Words { .. }
        | Operation::Closure { .. }
        | Operation::OrderedDfs { .. }
        | Operation::Cycles { .. }
        | Operation::Rows { .. }
        | Operation::Hosted { .. }
        | Operation::SpanBetween { .. }
        | Operation::RenderSyntax { .. }
        | Operation::FitsRendered { .. }
        | Operation::SpanBounds { .. } => return None,
    };
    comparisons
        .iter()
        .any(|comparison| match &comparison.right {
            Operand::Literal(QueryLiteral::Text(pattern))
                if comparison.test == ryo_values::domain::plan_parts::types::Test::Matches =>
            {
                invalid(pattern)
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

pub(super) fn test(
    tables: &mut PresenceTables,
    relation: Relation,
    comparisons: Vec<PlanComparison>,
) -> FormulaId {
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

/// The id of a set of provider span columns, each set kept once.
fn intern(tables: &mut PresenceTables, mut columns: Vec<SpanColumn>) -> LocatedId {
    columns.sort_unstable();
    columns.dedup();
    let columns = columns.into_boxed_slice();
    if let Some(found) = tables.located_index.get(&columns) {
        return *found;
    }
    let id = LocatedId(tables.located.len());
    tables.located.push(columns.clone());
    let _new = tables.located_index.insert(columns, id);
    id
}

/// The located columns of a per-file provider relation read: each span column.
pub(super) fn located_relation(
    tables: &mut PresenceTables,
    relation: Relation,
    kinds: &[CellKind],
) -> Vec<Option<LocatedId>> {
    kinds
        .iter()
        .enumerate()
        .map(|(column, kind)| {
            (*kind == CellKind::Span).then(|| {
                intern(
                    tables,
                    vec![SpanColumn {
                        relation,
                        column: ColumnIndex(u32::try_from(column).unwrap_or(u32::MAX)),
                    }],
                )
            })
        })
        .collect()
}

/// The located columns of a node's output: mapped through the shared column provenance, a union's column located only
/// when both of its sides are, and a source search's match and capture spans located when the span it searched was.
pub(super) fn located_output(
    tables: &mut PresenceTables,
    operation: &Operation,
    shapes: &[InputShape<'_>],
    inputs: &[&[Option<LocatedId>]],
) -> Vec<Option<LocatedId>> {
    let mut answer = output_origins(operation, shapes, inputs, &mut |_, located| located);
    match (operation, inputs) {
        (Operation::Union { .. }, [left, right]) => {
            answer = left
                .iter()
                .zip(right.iter().chain(std::iter::repeat(&None)))
                .map(|pair| match pair {
                    (Some(left), Some(right)) => {
                        let columns = [left, right]
                            .iter()
                            .filter_map(|id| tables.located.get(id.0))
                            .flat_map(|set| set.iter().copied())
                            .collect();
                        Some(intern(tables, columns))
                    }
                    _ => None,
                })
                .collect();
        }
        (Operation::Union { .. }, _) => answer.clear(),
        (
            Operation::SourceMatches {
                span,
                base_line,
                capture,
                ..
            },
            [input, ..],
        ) => {
            let searched = input.get(span.0 as usize).copied().flatten();
            if let Some(slot) = answer.get_mut(input.len()) {
                *slot = searched;
            }
            if capture.is_some()
                && let Some(slot) =
                    answer.get_mut(input.len() + 1 + usize::from(base_line.is_some()))
            {
                *slot = searched;
            }
        }
        _ => {}
    }
    answer
}

/// The provider span columns a source search relies on to declare no failure: its span is located, and its literal
/// pattern compiles, with the named capture it asks for, in the request's prepared slot, which keeps the compiled
/// matcher for the run. Any other search can fail on its rows.
pub(super) fn relied(
    tables: &PresenceTables,
    operation: &Operation,
    span: ColumnIndex,
    input: &[Option<LocatedId>],
    prepared: &PreparedMatchers,
    matcher: PatternMatcher<'_>,
) -> Option<Box<[SpanColumn]>> {
    let Operation::SourceMatches {
        pattern, capture, ..
    } = operation
    else {
        return None;
    };
    let located = input.get(span.0 as usize).copied().flatten()?;
    let Ok(regex) = prepared.regex(TextView(&pattern.0), matcher)? else {
        return None;
    };
    // The call the search makes refuses a name the matcher lacks before it reads any text.
    let refused = capture.as_ref().is_some_and(|name| {
        matcher
            .0
            .visit_regex_captures(
                regex,
                ryo_matching::inbound::use_cases::matching::types::SearchText(<&str>::default()),
                ryo_matching::inbound::use_cases::matching::types::GroupName(&name.0),
                &mut |_, _| std::ops::ControlFlow::Break(()),
            )
            .is_err()
    });
    if refused {
        return None;
    }
    tables.located.get(located.0).cloned()
}
