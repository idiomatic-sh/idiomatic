//! Static native lookup shape. Unsupported readable shapes decline, never become faults.
use super::types::{NativeFinding, NativeForm};
use ryo_values::domain::plan_parts::types::ColumnIndex;

fn source_column(view: &[ColumnIndex], column: ColumnIndex) -> ColumnIndex {
    view.get(usize::try_from(column.0).unwrap_or(usize::MAX))
        .copied()
        .unwrap_or(column)
}

/// The native lookup a join of these key columns over this view reduces to, when it reduces to one.
pub(crate) fn finding(
    form: NativeForm,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    view: &[ColumnIndex],
) -> Option<NativeFinding> {
    if let ([left_key], [right_key]) = (left_on, right_on) {
        let key = source_column(view, *right_key);
        if key == ColumnIndex(4) && form == NativeForm::Elements {
            return Some(NativeFinding::Span(*left_key));
        }
        if key == ColumnIndex(0) || key == ColumnIndex(1) && form == NativeForm::Edges {
            return Some(NativeFinding::Node(*left_key, key));
        }
    }
    if let ([left_file, left_stretch], [right_file, right_stretch]) = (left_on, right_on) {
        if form == NativeForm::Elements {
            return match (
                source_column(view, *right_file),
                source_column(view, *right_stretch),
            ) {
                (ColumnIndex(3), ColumnIndex(4)) => {
                    Some(NativeFinding::Stretch(*left_file, *left_stretch))
                }
                (ColumnIndex(4), ColumnIndex(3)) => {
                    Some(NativeFinding::Stretch(*left_stretch, *left_file))
                }
                _ => None,
            };
        }
    }
    None
}
