//! Logical execution-buffer retirement and an aggregate bounded reuse allowance.

use crate::domain::operating::types::Room;
use crate::domain::operator::types::Pool;
use crate::domain::table_model::types::Table;

/// Aggregate allowance for reusable execution buffers, distinct from live outputs/compiled plans.
pub(crate) const REUSE_BYTES: usize = 32 * 1024 * 1024;

/// Pool and invocation scratch share one reusable-payload allowance after their last row consumer.
pub(crate) fn retire(
    pool: &mut Pool,
    scratch: &mut crate::domain::table_scratch::types::OperatorScratch,
) {
    crate::domain::table_scratch::service::clear_scratch(scratch);
    clear_rule(pool);
    let mut bytes = crate::domain::table_scratch::service::scratch_bytes(scratch);
    if bytes > REUSE_BYTES {
        *scratch = Default::default();
        bytes = 0;
    }
    let retained = retain_pool(pool, REUSE_BYTES.saturating_sub(bytes));
    tracing::trace!(target: "ryo::diagnostic::working_set", event = "execution_retained",
        reusable_payload_bytes = retained.saturating_add(bytes), budget_bytes = REUSE_BYTES,
        hash_capacity_accounting = "conservative", live_outputs_excluded = true, compiled_plans_excluded = true);
}

/// Clear dead rule state after rendering, preserving frontier ownership and whole-file shared classes.
pub(crate) fn clear_rule(pool: &mut Pool) {
    use crate::domain::table_scratch::service::{clear_scratch, vector_bytes};
    let live_cells = pool
        .steps
        .iter()
        .map(|table| table.cells.len())
        .sum::<usize>();
    pool.selected
        .iter_mut()
        .for_each(|selection| *selection = None);
    for table in &mut pool.steps {
        table.cells.clear();
        table.count.0 = 0;
    }
    for room in pool.rooms.iter_mut().flatten() {
        for table in room.held.iter_mut().chain(std::iter::once(&mut room.out)) {
            table.cells.clear();
            table.count.0 = 0;
        }
        clear_scratch(&mut room.scratch);
        room.memo.clear();
        room.counts.0.clear();
        room.hashed.clear();
        room.pairs.clear();
        room.kinds.clear();
    }
    for row in &mut pool.spare {
        row.clear();
    }
    for positions in &mut pool.made.made {
        positions.slots.0.clear();
    }
    pool.made.ready.clear();
    pool.made.found.0.clear();
    pool.made.cells.clear();
    pool.made.keys.0.clear();
    pool.made.probe.0.clear();
    pool.made.order.0.clear();
    pool.made.keep.0.clear();
    let retained = retain_pool(pool, REUSE_BYTES);
    tracing::trace!(target: "ryo::diagnostic::working_set", event = "rule_released",
        live_cell_bytes = live_cells.saturating_mul(size_of::<ryo_language::domain::relations::types::Value>()),
        retained_execution_bytes = retained, reuse_budget_bytes = REUSE_BYTES,
        live_shared_position_bytes = pool.made.classes.iter().map(|slots| vector_bytes(&slots.0)).sum::<usize>(),
        hash_capacity_accounting = "conservative", compiled_plan_bytes_excluded = true);
}

/// Bound all eligible reusable slots together; no per-vector shrink churn and no eviction of live outputs.
pub(crate) fn retain_pool(pool: &mut Pool, limit: usize) -> usize {
    use crate::domain::table_scratch::service::{scratch_bytes, table_bytes, vector_bytes};
    let mut remaining = limit;
    for table in &mut pool.steps {
        let cost = table_bytes(table);
        if cost > remaining {
            *table = Table::default();
        } else {
            remaining -= cost;
        }
    }
    for (index, table) in pool.frontier.iter_mut().enumerate() {
        if pool.frontier_pending.get(index) == Some(&true) {
            continue;
        } // still-owned frontier outputs are required live memory
        let cost = table_bytes(table);
        if cost > remaining {
            *table = Table::default();
        } else {
            remaining -= cost;
        }
    }
    for slot in &mut pool.rooms {
        let Some(room) = slot else {
            continue;
        };
        let cost = room
            .held
            .iter()
            .chain(std::iter::once(&room.out))
            .map(table_bytes)
            .fold(size_of::<Room>(), usize::saturating_add)
            .saturating_add(scratch_bytes(&room.scratch))
            .saturating_add(vector_bytes(&room.memo))
            .saturating_add(vector_bytes(&room.kinds))
            .saturating_add(vector_bytes(&room.counts.0))
            .saturating_add(vector_bytes(&room.hashed))
            .saturating_add(
                room.pairs
                    .capacity()
                    .saturating_add(1)
                    .checked_next_power_of_two()
                    .unwrap_or(usize::MAX)
                    .saturating_mul(size_of::<crate::domain::operating::types::GroupValue>() + 1)
                    .saturating_add(64),
            );
        if cost > remaining {
            *slot = None;
        } else {
            remaining -= cost;
        }
    }
    for row in &mut pool.spare {
        retain_vec(row, &mut remaining);
    }
    for positions in &mut pool.made.made {
        retain_vec(&mut positions.slots.0, &mut remaining);
    }
    for class in &mut pool.made.classes {
        if class.0.is_empty() {
            retain_vec(&mut class.0, &mut remaining);
        }
    }
    retain_vec(&mut pool.made.found.0, &mut remaining);
    retain_vec(&mut pool.made.cells, &mut remaining);
    retain_vec(&mut pool.made.keys.0, &mut remaining);
    retain_vec(&mut pool.made.probe.0, &mut remaining);
    retain_vec(&mut pool.made.order.0, &mut remaining);
    retain_vec(&mut pool.made.keep.0, &mut remaining);
    limit - remaining
}

/// Drop a dead reusable allocation only when the aggregate allowance cannot retain it.
fn retain_vec<T>(values: &mut Vec<T>, remaining: &mut usize) {
    let bytes = crate::domain::table_scratch::service::vector_bytes(values);
    if bytes > *remaining {
        *values = Vec::new();
    } else {
        *remaining -= bytes;
    }
}
