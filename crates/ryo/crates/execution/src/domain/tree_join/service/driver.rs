//! One native-tree cursor retains the original kind, text and residual admission order.
use crate::domain::comparing::service::holds_some;
use crate::domain::kind_sets::service::passes;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, OperatorContext};
use crate::domain::table_cells::service::{
    kind_name_at, ordinal_number_at, read_tree_cells, span_at,
};
use crate::domain::table_cells::types::{CellsRead, TreeForm, TreeRows};
use crate::domain::table_model::service::kinds_of;
use crate::domain::table_model::types::{Rows, Verdict};
use crate::domain::table_scratch::types::ElementIndex;
use crate::domain::text_gate::service::text_admitted;
use crate::domain::tree_join::types::TreeState;
use ryo_language::domain::elements::service::{field_name_at, kind_at, parent_at, text_at};
use ryo_operators::inbound::use_cases::operators::interface::{JoinDriver, JoinInterrupted};
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{Flag, Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Comparison};
use ryo_values::domain::rows::types::RowIndex;
pub(super) struct Driver<'a, 'f, 'c> {
    #[cfg(feature = "diagnostic-row-progress")]
    pub(super) progress: crate::domain::operator::contract::views::ProgressObserver,
    pub(super) state: &'a mut TreeState,
    pub(super) work: &'a crate::domain::operator::contract::types::WorkCounters,
    pub(super) tree: &'a TreeRows<'f>,
    pub(super) comparisons: &'a [Comparison],
    pub(super) context: &'a mut OperatorContext<'c>,
    pub(super) emit: &'a mut dyn Emit,
    pub(super) at: usize,
    pub(super) current: Option<Position>,
    pub(super) error: Option<OperatingError>,
}
impl Driver<'_, '_, '_> {
    fn advance(&mut self) -> Result<bool, OperatingError> {
        let first = usize::from(self.tree.form == TreeForm::Edges);
        let tree_rows = Rows::Tree(*self.tree);
        let elements = kinds_of(&tree_rows);
        while let Some(found) = self.state.stretched.get(self.at).copied() {
            self.at += 1;
            self.work.found(1, 1);
            #[cfg(feature = "diagnostic-row-progress")]
            (self.progress)(
                crate::domain::operator::contract::views::ProgressCounter::JoinCandidatePairs,
                1,
            );
            if !self.state.kinds.tests.0.is_empty()
                && passes(&self.state.kinds, kind_at(self.tree.elements, found)) == Verdict::Fails
            {
                continue;
            }
            let index = RowIndex(usize::try_from(found.0).unwrap_or(usize::MAX) - first);
            #[cfg(feature = "diagnostic-row-progress")]
            if !self.state.gate.0.is_empty() {
                (self.progress)(
                    crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
                    1,
                );
            }
            if !self.state.gate.0.is_empty()
                && text_admitted(
                    self.tree,
                    ElementIndex(usize::try_from(found.0).unwrap_or(usize::MAX)),
                    self.comparisons,
                    &self.state.gate,
                    &mut self.state.text_ok,
                    self.state.names,
                    &mut self.state.cells,
                    elements,
                    self.context,
                )? == Verdict::Fails
            {
                continue;
            }
            #[cfg(feature = "diagnostic-row-progress")]
            if !self.state.rest.0.is_empty() {
                (self.progress)(
                    crate::domain::operator::contract::views::ProgressCounter::PredicateEvaluations,
                    1,
                );
            }
            if !self.state.rest.0.is_empty()
                && (read_tree_cells(self.tree, index, &mut self.state.cells, &self.state.needed)
                    == CellsRead::Missing
                    || holds_some(
                        self.comparisons,
                        &self.state.rest,
                        &self.state.cells,
                        elements,
                        self.state.names,
                        self.context,
                    )? == Verdict::Fails)
            {
                continue;
            }
            self.current = Some(found);
            return Ok(true);
        }
        self.current = None;
        Ok(false)
    }
}
impl JoinDriver for Driver<'_, '_, '_> {
    fn next(&mut self, _key: &[Value]) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.advance().map_err(|error| {
            self.error = Some(error);
            JoinInterrupted
        })
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        if self.error.is_some() {
            return None;
        }
        self.current
            .map(|position| tree_value(self.tree, position, column))
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.emit.row(self.context, row).map_err(|error| {
            self.error = Some(error);
            JoinInterrupted
        })
    }
}

/// One cell of the row of the tree at `at`: of the edge to the element, or of the element.
pub(crate) fn tree_value(tree: &TreeRows<'_>, at: Position, column: ColumnIndex) -> Value {
    let elements = tree.elements;
    if tree.form == TreeForm::Edges {
        return match column.0 {
            0 => Value::Node(node_id(tree.file, parent_at(elements, at))),
            1 => Value::Node(node_id(tree.file, at)),
            2 => Value::Number(ordinal_number_at(elements, at)),
            3 => Value::Symbol(field_name_at(elements, at)),
            4 => Value::File(tree.file),
            _ => Value::Flag(Flag::Lowered),
        };
    }
    match column.0 {
        0 => Value::Node(node_id(tree.file, at)),
        1 => Value::Symbol(kind_name_at(elements, at)),
        2 => Value::Symbol(text_at(elements, at)),
        3 => Value::File(tree.file),
        4 => Value::Span(span_at(elements, at)),
        _ => Value::Flag(Flag::Lowered),
    }
}
