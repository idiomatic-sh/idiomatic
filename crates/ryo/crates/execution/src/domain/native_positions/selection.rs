//! Borrowed selection and projection; all row storage remains with the caller.

use super::types::{PositionInput, PositionWork};
use crate::domain::{
    comparing::service::holds_all,
    lowering::{service::kind_of, types::LoweredShape},
    operating::{
        errors::OperatingError,
        types::{Memo, NameCount, OperatorContext},
    },
    operator::contract::types::WorkCounters,
    positions::{
        service::{count, row, values},
        types::{Positions, Slots},
    },
    table_cells::types::TreeRows,
    table_model::types::Verdict,
};
use ryo_derived::domain::raw_order::types::raw_order::value::RawOrder;
use ryo_language::domain::names::service::name_count;
use ryo_values::domain::{
    plan_parts::types::{Input, RepeatedRows},
    plans::types::Operation,
    rows::types::RowIndex,
};

pub(super) fn select(
    operation: &Operation,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    context: &mut OperatorContext<'_>,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Positions, OperatingError> {
    let Operation::Select {
        from, comparisons, ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    let mut slots = Slots::default();
    if matches!(from, Input::Relation(_)) {
        crate::domain::selections::service::select_tree_with_visits(
            tree,
            comparisons,
            context,
            &mut |_| counts.found(ryo_values::domain::plan_parts::types::InputNumber(0), crate::domain::operator::contract::types::FoundRows(1)),
            &mut slots,
        )?;
    } else {
        let source = inputs
            .first()
            .copied()
            .ok_or(OperatingError::MissingInput { position: 1 })?;
        let kinds = source
            .shape
            .columns
            .iter()
            .map(|column| kind_of(column.field))
            .collect::<Vec<_>>();
        let names = NameCount(name_count(context.symbols.as_ref()).0);
        context.memo.clear();
        context
            .memo
            .resize(comparisons.len() * names.0, Memo::Unknown);
        for at in positions(count(source.rows)) {
            counts.found(ryo_values::domain::plan_parts::types::InputNumber(0), crate::domain::operator::contract::types::FoundRows(1));
            let each = row(source.rows, at);
            values(tree, &source.shape.columns, each, &mut work.cells);
            if holds_all(comparisons, &work.cells, &kinds, names, context)? == Verdict::Holds {
                slots.0.extend_from_slice(each.0);
            }
        }
    }
    Ok(Positions {
        width: output.width,
        slots,
    })
}

pub(super) fn project(
    order: Option<&RawOrder>,
    operation: &Operation,
    tree: &TreeRows<'_>,
    inputs: &[PositionInput<'_>],
    output: &LoweredShape,
    work: &mut PositionWork,
    counts: &WorkCounters,
) -> Result<Positions, OperatingError> {
    let Operation::Project {
        columns, distinct, ..
    } = operation
    else {
        return Err(OperatingError::WrongOperation);
    };
    // The compiler declines this valid zero-column operation before native work.
    if columns.is_empty() {
        return Err(OperatingError::WrongOperation);
    }
    let source = inputs
        .first()
        .copied()
        .ok_or(OperatingError::MissingInput { position: 1 })?;
    let mut slots = Slots::default();
    if *distinct == RepeatedRows::Kept {
        counts.found(ryo_values::domain::plan_parts::types::InputNumber(0), crate::domain::operator::contract::types::FoundRows(count(source.rows).0 as u64));
        slots.0.extend_from_slice(&source.rows.slots.0);
    } else {
        let order = order.ok_or(OperatingError::WrongOperation)?;
        let PositionWork { kept, .. } = work;
        let mut flags = ryo_derived::domain::raw_words::types::KeepFlags(std::mem::take(&mut kept.0));
        order.firsts(&mut flags);
        kept.0 = flags.0;
        for (at, first) in kept.0.iter().enumerate() {
            if *first {
                slots.0.extend_from_slice(row(source.rows, RowIndex(at)).0);
            }
        }
    }
    Ok(Positions {
        width: output.width,
        slots,
    })
}

use ryo_values::domain::rows::service::positions;
