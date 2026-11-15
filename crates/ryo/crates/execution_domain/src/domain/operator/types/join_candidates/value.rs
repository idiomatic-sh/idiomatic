//! The `JoinCandidates` value: the join candidates of one left row (the native tree candidates when a native
//! lookup is bound, else the index probe), the engine's strategy behind the operators' join and set-row ports.

use super::super::row_delivery::value::RowDelivery;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::failure::types::LatchState;
use crate::domain::operator::contract::resources::types::Filtering;
use crate::domain::operator::contract::types::PrimitiveResources;
use crate::domain::operator::contract::views::types::{NativeCursor, NativeJoin, Started};
use crate::domain::table_cells::service::find_native;
use crate::domain::table_reach::borrowed::types::{IndexedRows, KeyProbe};
use ryo_operators::domain::contract::types::PredicateOutcome;
use ryo_operators::domain::contract::types::RightColumns;
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_operators::domain::equality_join::types::Advance;
use ryo_operators::domain::set_rows::errors::SetRowsError;
use ryo_operators::domain::set_rows::types::{RightMembership, RightPopulation};
use ryo_operators::domain::tree_cells::service::tree_value;
use ryo_relations::domain::relations::types::{Position, Value};
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::RowIndex;

/// The cursor over one left row's join candidates, with the positions buffer the workspace keeps.
pub(crate) struct JoinCandidates<'a, 'f> {
    left: &'a [Value],
    native: Option<NativeJoin<'f>>,
    index: Option<IndexedRows<'f>>,
    probe: Option<KeyProbe>,
    current: Option<RowIndex>,
    positions: Vec<Position>,
    at: NativeCursor,
    started: Started,
    position: Option<Position>,
    filtered: Filtering,
}

impl<'a, 'f> JoinCandidates<'a, 'f> {
    /// The candidates of `left` over the bound resources, with the workspace's positions buffer.
    pub(crate) fn new(
        resources: &'f dyn PrimitiveResources,
        operation: &Operation,
        left: &'a [Value],
        positions: Vec<Position>,
    ) -> Self {
        let filtered = if matches!(operation, Operation::Join { filter, .. } if !filter.is_empty())
        {
            Filtering::Filtered
        } else {
            Filtering::Unfiltered
        };
        Self {
            left,
            native: resources.native_join(),
            index: resources.index(),
            probe: None,
            current: None,
            positions,
            at: NativeCursor(0),
            started: Started::No,
            position: None,
            filtered,
        }
    }

    /// The positions buffer, handed back to the workspace.
    pub(crate) fn give_back(self) -> Vec<Position> {
        self.positions
    }

    /// Whether the right cells are the native tree's physical columns or the index's logical ones.
    pub(crate) fn right_columns(&self) -> RightColumns {
        if self.native.is_some() {
            RightColumns::Physical
        } else {
            RightColumns::Logical
        }
    }

    fn advance(
        &mut self,
        rows: &mut RowDelivery<'_, '_>,
        key: &[Value],
    ) -> Result<Advance, OperatingError> {
        if let Some(native) = self.native {
            if self.started == Started::No {
                self.started = Started::Yes;
                self.positions.clear();
                find_native(&native.tree, self.left, native.finding, &mut self.positions)?;
            }
            while let Some(position) = self.positions.get(self.at.0).copied() {
                self.at.0 += 1;
                rows.candidate_found();
                if self.filtered == Filtering::Filtered {
                    let row = std::array::from_fn::<_, 5, _>(|at| {
                        tree_value(
                            &native.tree,
                            position,
                            ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)),
                        )
                    });
                    if rows.verdict(&row)? == PredicateOutcome::Fails {
                        continue;
                    }
                }
                self.position = Some(position);
                return Ok(Advance::Admitted);
            }
            self.position = None;
            return Ok(Advance::Exhausted);
        }
        let index = self.index.ok_or(OperatingError::NotRun)?;
        let probe = self.probe.get_or_insert_with(|| {
            crate::domain::table_reach::borrowed::service::probe(&index, key)
        });
        self.current =
            crate::domain::table_reach::borrowed::service::next_match(&index, key, probe);
        if self.current.is_some() {
            rows.candidate_found();
        }
        Ok(Advance::from(self.current.is_some()))
    }

    /// The next candidate of the key.
    pub(crate) fn next(
        &mut self,
        rows: &mut RowDelivery<'_, '_>,
        key: &[Value],
    ) -> Result<Advance, EqualityJoinError> {
        if rows.interrupted() == LatchState::Failed {
            return Err(EqualityJoinError::Interrupted);
        }
        let advanced = self.advance(rows, key);
        rows.hold(advanced)
            .map_err(|_| EqualityJoinError::Interrupted)
    }

    /// One cell of the current candidate.
    pub(crate) fn right_cell(
        &self,
        rows: &RowDelivery<'_, '_>,
        column: ColumnIndex,
    ) -> Option<Value> {
        if rows.interrupted() == LatchState::Failed {
            return None;
        }
        if let (Some(native), Some(position)) = (self.native, self.position) {
            Some(tree_value(&native.tree, position, column))
        } else {
            self.current
                .and_then(|at| self.index?.rows.cell(at, column))
        }
    }

    /// Whether the right input is empty.
    pub(crate) fn right_empty(
        &self,
        rows: &mut RowDelivery<'_, '_>,
    ) -> Result<RightPopulation, SetRowsError> {
        if rows.interrupted() == LatchState::Failed {
            return Err(SetRowsError::Interrupted);
        }
        let empty = self
            .index
            .map(|index| RightPopulation::from(index.rows.count().0 == 0))
            .ok_or(OperatingError::WrongBinding);
        rows.hold(empty).map_err(|_| SetRowsError::Interrupted)
    }

    /// Whether the right input holds the key.
    pub(crate) fn contains_right(
        &self,
        rows: &mut RowDelivery<'_, '_>,
        key: &[Value],
    ) -> Result<RightMembership, SetRowsError> {
        if rows.interrupted() == LatchState::Failed {
            return Err(SetRowsError::Interrupted);
        }
        let contained = self
            .index
            .map(|index| crate::domain::table_reach::borrowed::service::contains(&index, key))
            .ok_or(OperatingError::WrongBinding);
        rows.hold(contained).map_err(|_| SetRowsError::Interrupted)
    }
}
