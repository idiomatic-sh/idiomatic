//! Native lookup is immutable semantic data, with no cache or service capabilities; the engine's join
//! candidates (an index probe, or the elements of a tree a left row finds) are its strategy behind the
//! operators' join driver port.
use super::driver::LeaseDriver;
use super::resources::Filtering;
use super::types::{FoundRows, PrimitiveResources};
use super::views::{IndexedRows, KeyProbe};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::service::{find_native, tree_value};
use crate::domain::table_cells::types::TreeRows;
use crate::domain::table_model::types::Verdict;
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_operators::domain::equality_join::types::{Advance, JoinDriver};
use ryo_relations::domain::relations::types::{Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, InputNumber};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::RowIndex;

use ryo_derived::domain::native::types::NativeFinding as Finding;
#[derive(Clone, Copy)]
pub(crate) struct NativeJoin<'a> {
    pub tree: TreeRows<'a>,
    pub finding: Finding,
}

/// Whether the native candidates of the left row have been found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Started {
    Yes,
    No,
}

/// Where the walk over the native candidates has reached.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PositionCursor(pub(super) usize);

/// The join candidates of one left row: the native tree candidates when a native lookup is bound, else the
/// index probe; a filtered native join admits only the elements its bound predicate holds for.
pub(crate) struct JoinCursor<'f> {
    native: Option<NativeJoin<'f>>,
    index: Option<IndexedRows<'f>>,
    probe: Option<KeyProbe>,
    current: Option<RowIndex>,
    positions: Vec<Position>,
    at: PositionCursor,
    started: Started,
    position: Option<Position>,
    filtered: Filtering,
}

impl<'f> JoinCursor<'f> {
    /// The cursor of an evaluation, with the positions buffer the workspace keeps.
    pub(super) fn new(
        resources: &'f dyn PrimitiveResources,
        operation: &Operation,
        positions: Vec<Position>,
    ) -> Self {
        let filtered = match operation {
            Operation::Join { filter, .. } if !filter.is_empty() => Filtering::Filtered,
            _ => Filtering::Unfiltered,
        };
        Self {
            native: resources.native_join(),
            index: resources.index(),
            probe: None,
            current: None,
            positions,
            at: PositionCursor(0),
            started: Started::No,
            position: None,
            filtered,
        }
    }

    /// The bound native lookup, if any.
    pub(super) fn native(&self) -> Option<NativeJoin<'f>> {
        self.native
    }

    /// The positions buffer, handed back to the workspace.
    pub(super) fn give_back(self) -> Vec<Position> {
        self.positions
    }
}

impl LeaseDriver<'_, '_> {
    fn advance(&mut self, key: &[Value]) -> Result<Advance, OperatingError> {
        if let Some(native) = self.join.native {
            if self.join.started == Started::No {
                self.join.started = Started::Yes;
                self.join.positions.clear();
                find_native(
                    &native.tree,
                    self.left(),
                    native.finding,
                    &mut self.join.positions,
                )?;
            }
            while let Some(position) = self.join.positions.get(self.join.at.0).copied() {
                self.join.at.0 += 1;
                self.lease.found(InputNumber(1), FoundRows(1));
                #[cfg(feature = "diagnostic-row-progress")]
                self.lease.progress(
                    super::views::ProgressCounter::JoinCandidatePairs,
                    super::views::ProgressAmount(1),
                );
                if self.join.filtered == Filtering::Filtered {
                    let row = std::array::from_fn::<_, 5, _>(|at| {
                        tree_value(
                            &native.tree,
                            position,
                            ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)),
                        )
                    });
                    if self.lease.holds(&row)? == Verdict::Fails {
                        continue;
                    }
                }
                self.join.position = Some(position);
                return Ok(Advance::Admitted);
            }
            self.join.position = None;
            return Ok(Advance::Exhausted);
        }
        let index = self.join.index.ok_or(OperatingError::NotRun)?;
        let probe = self.join.probe.get_or_insert_with(|| index.probe(key));
        self.join.current = index.next_match(key, probe);
        if self.join.current.is_some() {
            self.lease.found(InputNumber(1), FoundRows(1));
            #[cfg(feature = "diagnostic-row-progress")]
            self.lease.progress(
                super::views::ProgressCounter::JoinCandidatePairs,
                super::views::ProgressAmount(1),
            );
        }
        Ok(Advance::from(self.join.current.is_some()))
    }
}

impl JoinDriver for LeaseDriver<'_, '_> {
    fn next(&mut self, key: &[Value]) -> Result<Advance, EqualityJoinError> {
        if self.interrupted() {
            return Err(EqualityJoinError::Interrupted);
        }
        let advanced = self.advance(key);
        self.hold(advanced)
            .map_err(|_| EqualityJoinError::Interrupted)
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        if self.interrupted() {
            return None;
        }
        if let (Some(native), Some(position)) = (self.join.native, self.join.position) {
            Some(tree_value(&native.tree, position, column))
        } else {
            self.join
                .current
                .and_then(|at| self.join.index?.rows.cell(at, column))
        }
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), EqualityJoinError> {
        ryo_operators::domain::evaluation::types::InvocationDriver::emit(self, row)
            .map_err(|_| EqualityJoinError::Interrupted)
    }
}
