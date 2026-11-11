//! The `candidates` service: free functions named by verb over the domain's types, with no IO and no ports.
use crate::domain::operator::contract::driver::service::hold;
use crate::domain::operator::contract::driver::service::interrupted;
use crate::domain::operator::contract::driver::service::left;

use super::super::driver::types::LeaseDriver;
use super::super::resources::types::Filtering;
use super::super::types::{FoundRows, PrimitiveResources};
use super::super::views::{
    types::JoinCursor, types::NativeCursor, types::NativeJoin, types::Started,
};
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::service::{find_native, tree_value};
use crate::domain::table_model::types::Verdict;
use ryo_operators::domain::equality_join::errors::EqualityJoinError;
use ryo_operators::domain::equality_join::types::Advance;
use ryo_operators::domain::set_rows::errors::SetRowsError;
use ryo_operators::domain::set_rows::types::{RightMembership, RightPopulation};
use ryo_relations::domain::relations::types::{Position, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, InputNumber};
use ryo_values::domain::plans::types::Operation;

/// The cursor of an evaluation, with the positions buffer the workspace keeps.
pub(crate) fn join_cursor<'f>(
    resources: &'f dyn PrimitiveResources,
    operation: &Operation,
    positions: Vec<Position>,
) -> JoinCursor<'f> {
    let filtered = if matches!(operation, Operation::Join { filter, .. } if !filter.is_empty()) {
        Filtering::Filtered
    } else {
        Filtering::Unfiltered
    };
    JoinCursor {
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

/// The bound native lookup, if any.
pub(crate) fn native<'f>(cursor: &JoinCursor<'f>) -> Option<NativeJoin<'f>> {
    cursor.native
}

/// The positions buffer, handed back to the workspace.
pub(crate) fn give_back<'f>(cursor: JoinCursor<'f>) -> Vec<Position> {
    cursor.positions
}

pub(crate) fn advance(
    driver: &mut LeaseDriver<'_, '_>,
    key: &[Value],
) -> Result<Advance, OperatingError> {
    if let Some(native) = driver.join.native {
        if driver.join.started == Started::No {
            driver.join.started = Started::Yes;
            driver.join.positions.clear();
            find_native(
                &native.tree,
                left(driver),
                native.finding,
                &mut driver.join.positions,
            )?;
        }
        while let Some(position) = driver.join.positions.get(driver.join.at.0).copied() {
            driver.join.at.0 += 1;
            crate::domain::operator::contract::lease::service::found(
                driver.lease,
                InputNumber(1),
                FoundRows(1),
            );
            #[cfg(feature = "diagnostic-row-progress")]
            crate::domain::operator::contract::lease::service::progress(
                driver.lease,
                crate::domain::operating::types::ProgressCounter::JoinCandidatePairs,
                crate::domain::operating::types::ProgressAmount(1),
            );
            if driver.join.filtered == Filtering::Filtered {
                let row = std::array::from_fn::<_, 5, _>(|at| {
                    tree_value(
                        &native.tree,
                        position,
                        ColumnIndex(u32::try_from(at).unwrap_or(u32::MAX)),
                    )
                });
                if crate::domain::operator::contract::lease::service::holds(driver.lease, &row)?
                    == Verdict::Fails
                {
                    continue;
                }
            }
            driver.join.position = Some(position);
            return Ok(Advance::Admitted);
        }
        driver.join.position = None;
        return Ok(Advance::Exhausted);
    }
    let index = driver.join.index.ok_or(OperatingError::NotRun)?;
    let probe = driver
        .join
        .probe
        .get_or_insert_with(|| crate::domain::table_reach::borrowed::service::probe(&index, key));
    driver.join.current =
        crate::domain::table_reach::borrowed::service::next_match(&index, key, probe);
    if driver.join.current.is_some() {
        crate::domain::operator::contract::lease::service::found(
            driver.lease,
            InputNumber(1),
            FoundRows(1),
        );
        #[cfg(feature = "diagnostic-row-progress")]
        crate::domain::operator::contract::lease::service::progress(
            driver.lease,
            crate::domain::operating::types::ProgressCounter::JoinCandidatePairs,
            crate::domain::operating::types::ProgressAmount(1),
        );
    }
    Ok(Advance::from(driver.join.current.is_some()))
}

pub(crate) fn next(
    driver: &mut LeaseDriver<'_, '_>,
    key: &[Value],
) -> Result<Advance, EqualityJoinError> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(EqualityJoinError::Interrupted);
    }
    let advanced = advance(driver, key);
    hold(driver, advanced).map_err(|_| EqualityJoinError::Interrupted)
}

pub(crate) fn right_cell(driver: &LeaseDriver<'_, '_>, column: ColumnIndex) -> Option<Value> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return None;
    }
    if let (Some(native), Some(position)) = (driver.join.native, driver.join.position) {
        Some(tree_value(&native.tree, position, column))
    } else {
        driver
            .join
            .current
            .and_then(|at| driver.join.index?.rows.cell(at, column))
    }
}

pub(crate) fn right_empty(
    driver: &mut LeaseDriver<'_, '_>,
) -> Result<RightPopulation, SetRowsError> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(SetRowsError::Interrupted);
    }
    let empty = driver
        .resources
        .index()
        .map(|index| RightPopulation::from(index.rows.count().0 == 0))
        .ok_or(OperatingError::WrongBinding);
    hold(driver, empty).map_err(|_| SetRowsError::Interrupted)
}

pub(crate) fn contains_right(
    driver: &mut LeaseDriver<'_, '_>,
    key: &[Value],
) -> Result<RightMembership, SetRowsError> {
    if interrupted(driver) == super::super::failure::types::LatchState::Failed {
        return Err(SetRowsError::Interrupted);
    }
    let contained = driver
        .resources
        .index()
        .map(|index| crate::domain::table_reach::borrowed::service::contains(&index, key))
        .ok_or(OperatingError::WrongBinding);
    hold(driver, contained).map_err(|_| SetRowsError::Interrupted)
}
