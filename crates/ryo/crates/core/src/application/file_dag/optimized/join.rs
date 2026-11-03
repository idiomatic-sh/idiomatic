//! Engine-owned physical lookup for a prepared plain Join over complete held inputs.
//! Primitive output semantics remain the declared left/right projection; index
//! choice, cache admission, fallback and diagnostics stay at this application edge.

use crate::domain::derived_views::types::DerivedView;
use crate::domain::operating::{
    errors::OperatingError,
    types::{Emit, OperatorContext},
};
use crate::domain::operator::contract::resources::{JoinResources, ResourceProgram};
use crate::domain::operator::contract::types::{
    EvaluationMode, OperatorContract, PrimitiveSemantics, join_result,
};
use crate::domain::operator::contract::views::{BorrowedRows, IndexedRows};
use crate::domain::table_cells::types::RowIndex;
use crate::domain::table_model::{
    service::{
        cell_at, check_columns, count_of, kinds_of, matching_stored, positions, read_row,
        rectangular,
    },
    types::{RowCount, Rows},
};
use ryo_relations::domain::relations::{service::kinds, types::Value};
use ryo_operators::domain::contract::types::{JoinDriver, JoinInterrupted, OperatorCall};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input};

const SPARSE_PAIR_LIMIT: usize = 65_536;
const SPARSE_RIGHT_LIMIT: usize = 1_024;

mod acquisition;
mod diagnostic;
use acquisition::acquired_index;
use diagnostic::Counts;
pub(crate) use diagnostic::StreamJoinStats;

struct Work {
    left_row: Vec<Value>,
    output: Vec<Value>,
    key: Vec<Value>,
}
impl Work {
    fn take(context: &mut OperatorContext<'_>) -> Self {
        Self {
            left_row: std::mem::take(&mut context.scratch.row),
            output: std::mem::take(&mut context.scratch.other),
            key: std::mem::take(&mut context.scratch.join_scratch.key),
        }
    }
    fn restore(self, context: &mut OperatorContext<'_>) {
        context.scratch.row = self.left_row;
        context.scratch.other = self.output;
        context.scratch.join_scratch.key = self.key;
    }
}

struct Pairs<'a, 'f, 'c> {
    right: &'a Rows<'f>,
    rows: &'a [(RowIndex, RowIndex)],
    at: usize,
    current: Option<RowIndex>,
    context: &'a mut OperatorContext<'c>,
    emit: &'a mut dyn Emit,
    counts: &'a mut Counts,
    error: Option<OperatingError>,
}
impl JoinDriver for Pairs<'_, '_, '_> {
    fn next(&mut self, _key: &[Value]) -> Result<bool, JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        self.current = self.rows.get(self.at).map(|(_, right)| *right);
        self.at += usize::from(self.current.is_some());
        Ok(self.current.is_some())
    }
    fn right_cell(&self, column: ColumnIndex) -> Option<Value> {
        if self.error.is_some() {
            return None;
        }
        self.current
            .and_then(|row| cell_at(self.right, row, column))
    }
    fn emit(&mut self, row: &[Value]) -> Result<(), JoinInterrupted> {
        if self.error.is_some() {
            return Err(JoinInterrupted);
        }
        if self.counts.enabled {
            self.counts.hits = self.counts.hits.saturating_add(1);
        }
        if let Err(error) = self.emit.row(self.context, row) {
            self.error = Some(error);
            return Err(JoinInterrupted);
        }
        Ok(())
    }
}

/// The sparse-left candidate is a complete pair list before its first output;
/// a dense pair set falls back without changing left-major bag order.
fn sparse_left(
    semantics: &dyn PrimitiveSemantics,
    contract: &OperatorContract,
    mode: EvaluationMode,
    join: &JoinResources,
    logical_left: Input,
    logical_right: Input,
    left: &Rows<'_>,
    right: &Rows<'_>,
    admission: &super::super::workspace::InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    work: &mut Work,
    counts: &mut Counts,
) -> Option<Result<(), OperatingError>> {
    let (Input::Relation(relation), Rows::Stored(view)) = (logical_left, left) else {
        return None;
    };
    let (left_count, right_count) = (count_of(left).0, count_of(right).0);
    if !rectangular(*view)
        || view.kinds != kinds(relation)
        || left_count > u32::MAX as usize
        || right_count == 0
        || right_count > SPARSE_RIGHT_LIMIT
        || left_count < right_count.saturating_mul(16)
        || matches!(right, Rows::Tree(_) | Rows::Selected(_))
        || matches!(right,Rows::Stored(other) if !rectangular(*other))
        || matches!(right,Rows::Computed(table) if table.cells.len()!=table.count.0.saturating_mul(table.kinds.len()))
    {
        return None;
    }
    let eligible = {
        let owner = &admission.owner().derived;
        let complete_right = match right {
            Rows::Stored(other) => {
                matches!(logical_right, Input::Relation(source) if owner.relation_eligible(source) && other.kinds==kinds(source))
            }
            Rows::Computed(_) => true,
            _ => false,
        };
        owner.relation_eligible(relation)
            && complete_right
            && owner.can_retain_equality(relation, *view, &join.left_on)
    };
    if !eligible {
        return None;
    }
    if let Err(error) = check_columns(kinds_of(right), &join.right_on) {
        return Some(Err(error.into()));
    }
    let storage = match acquired_index(
        contract, mode, true, left, right, admission, context, counts, eligible,
    ) {
        Ok(storage) => storage,
        Err(error) => return Some(Err(error)),
    };
    let lookup = match storage.view() {
        Ok(DerivedView::Equality(index)) => index,
        Ok(_) => return Some(Err(OperatingError::WrongBinding)),
        Err(error) => return Some(Err(error)),
    };
    let mut pairs = Vec::new();
    for right_at in positions(RowCount(right_count)) {
        work.key.clear();
        for column in &join.right_on {
            let Some(value) = cell_at(right, right_at, *column) else {
                return None;
            };
            work.key.push(value);
        }
        if counts.enabled {
            counts.probes = counts.probes.saturating_add(1);
        }
        for left_at in matching_stored(*view, &join.left_on, lookup, &work.key) {
            if pairs.len() == SPARSE_PAIR_LIMIT {
                return None;
            }
            pairs.push((*left_at, right_at));
        }
    }
    pairs.sort_unstable();
    counts.sparse_left = true;
    let mut start = 0;
    while start < pairs.len() {
        let left_at = pairs[start].0;
        let end = start + pairs[start..].partition_point(|(at, _)| *at == left_at);
        let Some(left_row) = read_row(left, left_at, &mut work.left_row) else {
            return Some(Err(OperatingError::ShapeMismatch));
        };
        let mut driver = Pairs {
            right,
            rows: &pairs[start..end],
            at: 0,
            current: None,
            context,
            emit,
            counts,
            error: None,
        };
        let answer = semantics.operators(OperatorCall::EqualityJoin {
            input: ryo_operators::domain::contract::types::JoinInput::Values {
                left: left_row,
                left_on: &join.left_on,
                output: &join.output,
                view: &[],
                driver: &mut driver,
                key: &mut work.key,
                target: &mut work.output,
            },
        });
        if let Err(error) = join_result(answer, driver.error) {
            return Some(Err(error));
        }
        start = end;
    }
    Some(Ok(()))
}

fn ordinary(
    semantics: &dyn PrimitiveSemantics,
    contract: &OperatorContract,
    mode: EvaluationMode,
    join: &JoinResources,
    logical_right: Input,
    left: &Rows<'_>,
    right: &Rows<'_>,
    admission: &super::super::workspace::InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
    work: &mut Work,
    counts: &mut Counts,
) -> Result<(), OperatingError> {
    let eligible = matches!((logical_right, right), (Input::Relation(relation), Rows::Stored(view)) if rectangular(*view) && admission.owner().derived.relation_eligible(relation));
    let storage = acquired_index(
        contract, mode, false, left, right, admission, context, counts, eligible,
    )?;
    let DerivedView::Equality(lookup) = storage.view()? else {
        return Err(OperatingError::WrongBinding);
    };
    let index = IndexedRows {
        rows: BorrowedRows::from(right),
        columns: &join.right_on,
        index: lookup,
    };
    for left_at in positions(count_of(left)) {
        let Some(left_row) = read_row(left, left_at, &mut work.left_row) else {
            continue;
        };
        if counts.enabled {
            counts.probes = counts.probes.saturating_add(1);
        }
        let mut counted = crate::domain::replay::types::Pipe(
            |context: &mut OperatorContext<'_>, row: &[Value]| {
                if counts.enabled {
                    counts.hits = counts.hits.saturating_add(1);
                }
                emit.row(context, row)
            },
        );
        super::super::equality_join::indexed(
            semantics,
            index,
            left_row,
            &join.left_on,
            &join.output,
            &mut work.key,
            &mut work.output,
            context,
            &mut counted,
        )?;
    }
    Ok(())
}

/// None leaves native, reach and opaque cases on the mandatory common path.
/// The owner calls this after static/empty/schema gates and before ordinary
/// right-index acquisition, preserving old sparse-left admission order.
pub(crate) fn supports(program: &ResourceProgram, logical: &[Input], rows: &[Rows<'_>]) -> bool {
    let ResourceProgram::Join(join) = program else {
        return false;
    };
    let [left, right] = rows else {
        return false;
    };
    logical.len() >= 2
        && join.reach.is_none()
        && !join.has_filter
        && join.view.is_empty()
        && !matches!(left, Rows::Tree(_))
        && !matches!(right, Rows::Tree(_))
}

pub(crate) fn evaluate(
    semantics: &dyn PrimitiveSemantics,
    contract: &OperatorContract,
    mode: EvaluationMode,
    program: &ResourceProgram,
    logical: &[Input],
    rows: &[Rows<'_>],
    admission: &super::super::workspace::InvocationWorkspace,
    context: &mut OperatorContext<'_>,
    emit: &mut dyn Emit,
) -> Option<Result<(), OperatingError>> {
    if !supports(program, logical, rows) {
        return None;
    }
    let ResourceProgram::Join(join) = program else {
        return None;
    };
    let [left, right] = rows else {
        return None;
    };
    let [logical_left, logical_right, ..] = logical else {
        return None;
    };
    let mut counts = Counts {
        enabled: tracing::enabled!(target:"ryo::diagnostic::stored_join",tracing::Level::TRACE),
        ..Counts::default()
    };
    if count_of(left).0 == 0 || count_of(right).0 == 0 {
        counts.finish(right, &join.right_on, *logical_right, "stored-key", true);
        return Some(Ok(()));
    }
    let mut work = Work::take(context);
    let sparse = sparse_left(
        semantics,
        contract,
        mode,
        join,
        *logical_left,
        *logical_right,
        left,
        right,
        admission,
        context,
        emit,
        &mut work,
        &mut counts,
    );
    let result = sparse.unwrap_or_else(|| {
        ordinary(
            semantics,
            contract,
            mode,
            join,
            *logical_right,
            left,
            right,
            admission,
            context,
            emit,
            &mut work,
            &mut counts,
        )
    });
    let path = if counts.sparse_left {
        "sparse-left-key"
    } else if matches!(right, Rows::Selected(_))
        || matches!(right,Rows::Stored(view) if rectangular(*view))
    {
        "stored-key"
    } else {
        "materialized"
    };
    counts.finish(right, &join.right_on, *logical_right, path, result.is_ok());
    work.restore(context);
    Some(result)
}
