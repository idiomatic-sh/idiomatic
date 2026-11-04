//! Borrowed selection and projection; all row storage remains with the caller.

use super::types::{PositionInput, PositionWork};
use crate::domain::{
    comparing::service::holds_all,
    lowering::{service::kind_of, types::LoweredShape},
    operating::{
        errors::OperatingError,
        types::{Memo, NameCount, OperatorContext},
    },
    operator::contract::{derived::ScopedDerived, types::WorkCounters},
    positions::{
        service::{count, key_into, row, values},
        types::{Keys, Positions, Slots},
    },
    table_cells::types::TreeRows,
    table_model::{service::positions, types::Verdict},
};
use ryo_language::domain::symbols::service::name_count;
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
        crate::domain::selections::service::select_tree(tree, comparisons, context, &mut slots)?;
        counts.found(0, tree.elements.kind.len() as u64);
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
            counts.found(0, 1);
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
    derived: &mut dyn ScopedDerived,
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
    let source = inputs
        .first()
        .copied()
        .ok_or(OperatingError::MissingInput { position: 1 })?;
    let mut slots = Slots::default();
    if *distinct == RepeatedRows::Kept {
        counts.found(0, count(source.rows).0 as u64);
        slots.0.extend_from_slice(&source.rows.slots.0);
    } else {
        let PositionWork { keys, kept, .. } = work;
        keys.0.clear();
        let mut keyed = Keys::default();
        for at in positions(count(source.rows)) {
            counts.found(0, 1);
            key_into(
                tree,
                &source.shape.columns,
                row(source.rows, at),
                columns,
                &mut keyed,
            );
        }
        std::mem::swap(&mut keys.0, &mut keyed.0);
        let result = derived.with_raw_order(&keys.0, columns.len().max(1), &mut |order| {
            order.firsts(&mut kept.0);
            for (at, first) in kept.0.iter().enumerate() {
                if *first {
                    slots.0.extend_from_slice(row(source.rows, RowIndex(at)).0);
                }
            }
            Ok(())
        });
        result?;
    }
    Ok(Positions {
        width: output.width,
        slots,
    })
}
