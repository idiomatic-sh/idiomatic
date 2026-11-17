//! The `plan_parts` service: free functions named by verb over the numbers a plan is made of, with no IO and no ports.

use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::service::{columns, relation_of};
use ryo_relations::domain::relations::types::{Column, Relation};

use super::types::{
    Applied, ColumnIndex, Family, FixedPrefix, Operand, ParameterId, PlanComparison, QueryLiteral,
    RowWidth, StepId, Test, TreeLookupKey,
};

/// The one of `items` that the step `id` names, when there is one.
#[must_use]
pub fn step_in<T>(items: &[T], id: StepId) -> Option<&T> {
    items.get(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// The one of `items` that the column `id` names, when there is one.
#[must_use]
pub fn column_in<T>(items: &[T], id: ColumnIndex) -> Option<&T> {
    items.get(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// The place of a relation's column among the relation's columns, from zero, in the order its item declares them.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
/// use ryo_relations::domain::relations::types::Column;
/// use ryo_values::domain::plan_parts::service::column_place;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// assert_eq!(column_place(Column::SyntaxTree(SyntaxTreeColumn::Id)), ColumnIndex(0));
/// ```
#[must_use]
pub fn column_place(column: Column) -> ColumnIndex {
    let place = columns(relation_of(column))
        .iter()
        .position(|each| *each == column)
        .unwrap_or(usize::MAX);
    ColumnIndex(u32::try_from(place).unwrap_or(u32::MAX))
}

/// The column of `relation` at `index`, when the relation has that many columns.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
/// use ryo_relations::domain::relations::types::{Column, Relation};
/// use ryo_values::domain::plan_parts::service::relation_column;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// assert_eq!(
///     relation_column(Relation::SyntaxTree, ColumnIndex(0)),
///     Some(Column::SyntaxTree(SyntaxTreeColumn::Id))
/// );
/// ```
#[must_use]
pub fn relation_column(relation: Relation, index: ColumnIndex) -> Option<Column> {
    column_in(&columns(relation), index).copied()
}

/// The column of a step's input that the column `column` of its view reads: the column itself when the view is empty, since the step
/// then reads its input whole; else the column the view keeps at that place. A column past a non-empty view is none: the view does not
/// make it, so nothing reads it.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::service::view_source;
/// use ryo_values::domain::plan_parts::types::ColumnIndex;
/// let view = [ColumnIndex(0), ColumnIndex(3)];
/// assert_eq!(view_source(&view, ColumnIndex(1)), Some(ColumnIndex(3)));
/// assert_eq!(view_source(&view, ColumnIndex(2)), None);
/// assert_eq!(view_source(&[], ColumnIndex(2)), Some(ColumnIndex(2)));
/// ```
#[must_use]
pub fn view_source(view: &[ColumnIndex], column: ColumnIndex) -> Option<ColumnIndex> {
    if view.is_empty() {
        Some(column)
    } else {
        column_in(view, column).copied()
    }
}

/// What a lookup into `relation` keyed on its columns `keys`, in order, finds its rows by, when the engine looks the syntax tree or
/// its edges up where they are: an element by its id or its span, or by its file and span; an edge by its parent or its child.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::Relation;
/// use ryo_values::domain::plan_parts::service::tree_lookup_key;
/// use ryo_values::domain::plan_parts::types::{ColumnIndex, TreeLookupKey};
/// assert_eq!(tree_lookup_key(Relation::SyntaxTree, &[ColumnIndex(4)]), Some(TreeLookupKey::Span));
/// assert_eq!(tree_lookup_key(Relation::SyntaxChild, &[ColumnIndex(3)]), None);
/// ```
#[must_use]
pub fn tree_lookup_key(relation: Relation, keys: &[ColumnIndex]) -> Option<TreeLookupKey> {
    let read: Vec<Option<Column>> = keys
        .iter()
        .map(|key| relation_column(relation, *key))
        .collect();
    match read.as_slice() {
        [
            Some(
                node @ (Column::SyntaxTree(SyntaxTreeColumn::Id)
                | Column::SyntaxChild(SyntaxChildColumn::Parent | SyntaxChildColumn::Child)),
            ),
        ] => Some(TreeLookupKey::Node(*node)),
        [Some(Column::SyntaxTree(SyntaxTreeColumn::Span))] => Some(TreeLookupKey::Span),
        [
            Some(Column::SyntaxTree(SyntaxTreeColumn::File)),
            Some(Column::SyntaxTree(SyntaxTreeColumn::Span)),
        ] => Some(TreeLookupKey::FileThenSpan),
        [
            Some(Column::SyntaxTree(SyntaxTreeColumn::Span)),
            Some(Column::SyntaxTree(SyntaxTreeColumn::File)),
        ] => Some(TreeLookupKey::SpanThenFile),
        _ => None,
    }
}

/// The one of `items` that the parameter `id` names, when there is one.
#[must_use]
pub fn parameter_in<T>(items: &[T], id: ParameterId) -> Option<&T> {
    items.get(usize::try_from(id.0).unwrap_or(usize::MAX))
}

/// Calls `visit` with each column an operand reads: the column itself, or the columns the arguments of its function read.
pub fn each_column<F>(operand: &Operand, visit: &mut F)
where
    F: FnMut(ColumnIndex),
{
    match operand {
        Operand::Column(column) => visit(*column),
        Operand::Function(applied) => {
            for argument in &applied.arguments {
                each_column(argument, visit);
            }
        }
        Operand::Parameter(_) | Operand::Literal(_) => {}
    }
}

/// Calls `visit` with each column an operand reads, to be renamed.
pub fn each_column_mut<F>(operand: &mut Operand, visit: &mut F)
where
    F: FnMut(&mut ColumnIndex),
{
    match operand {
        Operand::Column(column) => visit(column),
        Operand::Function(applied) => {
            for argument in &mut applied.arguments {
                each_column_mut(argument, visit);
            }
        }
        Operand::Parameter(_) | Operand::Literal(_) => {}
    }
}

/// The columns an operand reads, itself or through the arguments of its function, in the order written.
#[must_use]
pub fn columns_of(operand: &Operand) -> Vec<ColumnIndex> {
    let mut read = Vec::new();
    each_column(operand, &mut |column| read.push(column));
    read
}

/// The operand with each column it reads put where `remap` says; nothing when `remap` has no place for one of them.
pub fn remapped<F>(operand: &Operand, remap: &F) -> Option<Operand>
where
    F: Fn(ColumnIndex) -> Option<ColumnIndex>,
{
    match operand {
        Operand::Column(column) => remap(*column).map(Operand::Column),
        Operand::Function(applied) => Some(Operand::Function(Applied {
            function: applied.function,
            arguments: applied
                .arguments
                .iter()
                .map(|argument| remapped(argument, remap))
                .collect::<Option<Vec<_>>>()?,
        })),
        other @ (Operand::Parameter(_) | Operand::Literal(_)) => Some(other.clone()),
    }
}

/// The comparison with each column it tests put where `remap` says; nothing when `remap` has no place for one of them. The one
/// rename of a comparison: the planner's fusing and placement, the lookups it pushes and the rule compiler's observations.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::service::remapped_comparison;
/// use ryo_values::domain::plan_parts::types::{ColumnIndex, ComparisonOrigin, Operand, PlanComparison, Test};
/// let same = PlanComparison {
///     test: Test::Equal,
///     left: Operand::Column(ColumnIndex(0)),
///     right: Operand::Column(ColumnIndex(2)),
///     origin: ComparisonOrigin::Unrecorded,
/// };
/// let moved = remapped_comparison(&same, &|column| Some(ColumnIndex(column.0 + 1)));
/// assert_eq!(moved.map(|each| each.right), Some(Operand::Column(ColumnIndex(3))));
/// assert!(remapped_comparison(&same, &|column| (column.0 == 0).then_some(column)).is_none());
/// ```
pub fn remapped_comparison<F>(comparison: &PlanComparison, remap: &F) -> Option<PlanComparison>
where
    F: Fn(ColumnIndex) -> Option<ColumnIndex>,
{
    Some(PlanComparison {
        test: comparison.test,
        left: remapped(&comparison.left, remap)?,
        right: remapped(&comparison.right, remap)?,
        origin: comparison.origin,
    })
}

/// The family of comparison a test belongs to.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::service::family_of;
/// use ryo_values::domain::plan_parts::types::{Family, Test};
/// assert_eq!(family_of(Test::NotGlobAny), Family::Glob);
/// assert_eq!(family_of(Test::Matches), Family::Regex);
/// ```
#[must_use]
pub fn family_of(test: Test) -> Family {
    match test {
        Test::Equal | Test::NotEqual => Family::Equality,
        Test::NaturalLessThan
        | Test::LessThan
        | Test::LessThanOrEqual
        | Test::GreaterThan
        | Test::GreaterThanOrEqual => Family::Ordering,
        Test::FloatTextGreaterThanOrEqual => Family::FloatText,
        Test::NumericGreaterThanOrEqual | Test::IntegerSuccessor => Family::Numeric,
        Test::In | Test::NotIn => Family::Membership,
        Test::GlobAny | Test::NotGlobAny => Family::Glob,
        Test::StartsWith | Test::EndsWith => Family::Affix,
        Test::Matches => Family::Regex,
    }
}

/// The leading comparisons of `comparisons` that cannot fail over a row of `width` columns: a column of the row compared with
/// a written text by `=`, `!=`, `starts-with` or `ends-with`. The first comparison that may fail (a column past the row, a
/// parameter, a function, a pattern compiled from the row) ends it, so a fault is never moved behind a test that decides
/// without it. The one definition the planner's placement, the keyed-select rewrite and the engine's root kinds stop at.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::plan_parts::service::fixed_prefix;
/// use ryo_values::domain::plan_parts::types::{
///     ColumnIndex, ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, RowWidth, Test,
/// };
/// let written = |test, at| PlanComparison {
///     test,
///     left: Operand::Column(ColumnIndex(at)),
///     right: Operand::Literal(QueryLiteral::Text(LiteralText("n".into()))),
///     origin: ComparisonOrigin::Unrecorded,
/// };
/// let comparisons = [written(Test::Equal, 0), written(Test::Matches, 1), written(Test::Equal, 1)];
/// assert_eq!(fixed_prefix(&comparisons, RowWidth(2)).0.len(), 1);
/// ```
#[must_use]
pub fn fixed_prefix(comparisons: &[PlanComparison], width: RowWidth) -> FixedPrefix<'_> {
    let infallible = |comparison: &PlanComparison| {
        matches!(family_of(comparison.test), Family::Equality | Family::Affix)
            && match (&comparison.left, &comparison.right) {
                (Operand::Column(column), Operand::Literal(QueryLiteral::Text(_)))
                | (Operand::Literal(QueryLiteral::Text(_)), Operand::Column(column)) => {
                    usize::try_from(column.0).is_ok_and(|at| at < width.0)
                }
                _ => false,
            }
    };
    let length = comparisons
        .iter()
        .position(|comparison| !infallible(comparison))
        .unwrap_or(comparisons.len());
    FixedPrefix(comparisons.get(..length).unwrap_or_default())
}

/// The right comparisons `comparisons`, of the relation's columns, that a join of `relation` on its columns `right_on`, read
/// through `view`, decides in the probe: all of them exactly when the probe walks the tree, which is when the keys read one
/// node column (`tree_lookup_key` answers `TreeLookupKey::Node`) and every comparison is in `fixed_prefix`, so none can fail;
/// none (`None`) anywhere else, where they are a filter of the right input made before the join. The one walk condition: the
/// planner's fusing, placement and pushed lookups place a right predicate by it, and the operators' `tree_probe` rewrite
/// walks an unordered probe by it.
///
/// # Examples
///
/// ```
/// use ryo_relations::domain::relations::types::Relation;
/// use ryo_values::domain::plan_parts::service::rides_the_walk;
/// use ryo_values::domain::plan_parts::types::{
///     ColumnIndex, ComparisonOrigin, LiteralText, Operand, PlanComparison, QueryLiteral, Test,
/// };
/// let kind = |test| PlanComparison {
///     test,
///     left: Operand::Column(ColumnIndex(1)),
///     right: Operand::Literal(QueryLiteral::Text(LiteralText("ident".into()))),
///     origin: ComparisonOrigin::Unrecorded,
/// };
/// let by_id = [ColumnIndex(0)];
/// let walked = [kind(Test::Equal)];
/// assert!(rides_the_walk(Relation::SyntaxTree, &by_id, &[], &walked).is_some());
/// assert!(rides_the_walk(Relation::SyntaxTree, &by_id, &[], &[kind(Test::Matches)]).is_none());
/// // A span finds the element in an index, never by walking.
/// assert!(rides_the_walk(Relation::SyntaxTree, &[ColumnIndex(4)], &[], &walked).is_none());
/// ```
#[must_use]
pub fn rides_the_walk<'a>(
    relation: Relation,
    right_on: &[ColumnIndex],
    view: &[ColumnIndex],
    comparisons: &'a [PlanComparison],
) -> Option<FixedPrefix<'a>> {
    // The columns of `relation` that the keys read through the view.
    let read: Option<Vec<ColumnIndex>> =
        right_on.iter().map(|key| view_source(view, *key)).collect();
    let by_a_node = read.is_some_and(|read| {
        matches!(
            tree_lookup_key(relation, &read),
            Some(TreeLookupKey::Node(_))
        )
    });
    let prefix = fixed_prefix(comparisons, RowWidth(columns(relation).len()));
    (by_a_node && prefix.0.len() == comparisons.len()).then_some(prefix)
}
