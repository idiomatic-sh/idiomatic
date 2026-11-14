//! Static native lookup shape. Unsupported readable shapes decline, never become faults.
use super::types::{NativeFinding, NativeForm};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::service::{column_place, tree_lookup_key};
use ryo_values::domain::plan_parts::types::{ColumnIndex, TreeLookupKey};

fn source_column(view: &[ColumnIndex], column: ColumnIndex) -> ColumnIndex {
    match view.get(usize::try_from(column.0).unwrap_or(usize::MAX)) {
        Some(mapped) => *mapped,
        None => column,
    }
}

/// The native lookup a join of these key columns over this view reduces to, when it reduces to one: the tree's own lookup shapes,
/// as `tree_lookup_key` says them for the relation the form is.
pub(crate) fn finding(
    form: NativeForm,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    view: &[ColumnIndex],
) -> Option<NativeFinding> {
    let relation = match form {
        NativeForm::Elements => Relation::SyntaxTree,
        NativeForm::Edges => Relation::SyntaxChild,
    };
    let keys: Vec<ColumnIndex> = right_on
        .iter()
        .map(|key| source_column(view, *key))
        .collect();
    match (tree_lookup_key(relation, &keys)?, left_on) {
        (TreeLookupKey::Node(column), [left_key]) => {
            Some(NativeFinding::Node(*left_key, column_place(column)))
        }
        (TreeLookupKey::Span, [left_key]) => Some(NativeFinding::Span(*left_key)),
        (TreeLookupKey::FileThenSpan, [left_file, left_stretch])
        | (TreeLookupKey::SpanThenFile, [left_stretch, left_file]) => {
            Some(NativeFinding::Stretch(*left_file, *left_stretch))
        }
        _ => None,
    }
}
