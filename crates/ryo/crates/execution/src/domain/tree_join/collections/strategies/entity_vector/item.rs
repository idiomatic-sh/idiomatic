//! The `entity_vector` strategy: the elements of a tree are found by the entity a left row holds, which is where in the vector of
//! the elements the element is, so that no table of them is made and no key is hashed.

use ryo_language::domain::elements::service::children;
use ryo_relations::domain::relations::service::{node_file, node_position};
use ryo_relations::domain::relations::types::{Position, Value};

use super::super::interface::StrategiesItem;
use super::super::types::NodeColumns;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::tree_join::collections::strategies::types::Fitting;
use ryo_derived::domain::native::types::NativeFinding as Finding;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::service::cell;

/// `entity_vector`, one item of the `strategies` collection.
pub(crate) struct EntityVectorItem;

impl StrategiesItem for EntityVectorItem {
    fn applies(&self, finding: Option<Finding>) -> Fitting {
        if matches!(finding, Some(Finding::Node(..))) {
            Fitting::Fits
        } else {
            Fitting::Misfits
        }
    }
}

impl EntityVectorItem {
    /// Pushes onto `found` the elements the node of the left row at `left_key` names: the element itself, the child an edge leads
    /// to, or, when the key is the parent of the edges, the children of the node. A node of another file names none.
    ///
    /// # Errors
    ///
    /// `NoSuchColumn` when the row lacks the column.
    pub(crate) fn find(
        tree: &TreeRows<'_>,
        left_row: &[Value],
        columns: NodeColumns,
        found: &mut Vec<Position>,
    ) -> Result<(), OperatingError> {
        let NodeColumns {
            left: left_key,
            key,
        } = columns;
        let Value::Node(start) = cell(left_row, left_key)? else {
            return Ok(());
        };
        let at = node_position(start);
        if node_file(start) != tree.file
            || usize::try_from(at.0).map_or(true, |place| place >= tree.elements.kind.len())
        {
            return Ok(());
        }
        // An edge has a parent but the root: the edges of the root's children are found by it, no edge leads to the root.
        match tree.form {
            TreeForm::Edges if key == ColumnIndex(0) => found.extend(children(tree.elements, at)),
            TreeForm::Edges if at.0 == 0 => {}
            TreeForm::Edges | TreeForm::Elements => found.push(at),
        }
        Ok(())
    }
}
