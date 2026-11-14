//! The `plan_parts` service: free functions named by verb over the numbers a plan is made of, with no IO and no ports.

use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::service::{columns, relation_of};
use ryo_relations::domain::relations::types::{Column, Relation};

use super::types::{
    Applied, BindingPlace, ColumnIndex, ObservationBindingId, Operand, ParameterId, StepId,
    TreeLookupKey,
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

/// The place of an observation binding among the bindings of a request.
#[must_use]
pub fn binding_place(id: ObservationBindingId) -> BindingPlace {
    BindingPlace(usize::try_from(id.0).unwrap_or(usize::MAX))
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
