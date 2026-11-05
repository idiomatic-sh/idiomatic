//! The `span_index` strategy: the elements of a tree are found by the stretch of the file a left row holds. The elements are in
//! the order of the text, so those that start where the stretch does are together, found by a search in the starts.

use crate::domain::table_cells::service::span_at;
use ryo_relations::domain::relations::types::{Position, Value};

use super::super::interface::StrategiesItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::TreeRows;
use crate::domain::tree_join::collections::strategies::types::Fitting;
use ryo_derived::domain::contract::types::Finding;
use ryo_values::domain::rows::service::cell;

/// `span_index`, one item of the `strategies` collection.
pub(crate) struct SpanIndexItem;

impl StrategiesItem for SpanIndexItem {
    fn applies(&self, finding: Option<Finding>) -> Fitting {
        if matches!(finding, Some(Finding::Stretch(..) | Finding::Span(_))) {
            Fitting::Fits
        } else {
            Fitting::Misfits
        }
    }
}

impl SpanIndexItem {
    /// Pushes onto `found` the elements that cover exactly the stretch the left row holds: of a file it names (`Stretch`), or of the
    /// file being read (`Span`). A finding by node finds none here.
    ///
    /// # Errors
    ///
    /// `NoSuchColumn` when the row lacks a column.
    pub(crate) fn find(
        tree: &TreeRows<'_>,
        left_row: &[Value],
        finding: Finding,
        found: &mut Vec<Position>,
    ) -> Result<(), OperatingError> {
        let span = match finding {
            Finding::Stretch(file, stretch) => {
                let (Value::File(of), Value::Span(span)) =
                    (cell(left_row, file)?, cell(left_row, stretch)?)
                else {
                    return Ok(());
                };
                if of != tree.file {
                    return Ok(());
                }
                span
            }
            Finding::Span(stretch) => {
                let Value::Span(span) = cell(left_row, stretch)? else {
                    return Ok(());
                };
                span
            }
            Finding::Node(..) => return Ok(()),
        };
        if ryo_language::domain::elements::service::source_order(tree.elements)
            == ryo_language::domain::elements::types::SourceOrder::Reordered
        {
            found.extend(
                ryo_language::domain::elements::service::positions(tree.elements).filter(|at| {
                    let element = span_at(tree.elements, *at);
                    element.start == span.start && element.end == span.end
                }),
            );
            return Ok(());
        }
        let starts = &tree.elements.start;
        let from = starts.partition_point(|start| *start < span.start);
        for place in from..starts.len() {
            if starts.get(place) != Some(&span.start) {
                break;
            }
            if tree.elements.end.get(place) == Some(&span.end) {
                found.push(Position(u32::try_from(place).unwrap_or(u32::MAX)));
            }
        }
        Ok(())
    }
}
