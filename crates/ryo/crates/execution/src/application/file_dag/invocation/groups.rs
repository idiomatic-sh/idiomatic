//! Existing grouping semantic accumulator; input driving remains in the common engine.
use crate::application::file_dag::invocation::prepared_use::service::prepared;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::operator::contract::types::OperatorContract;

use crate::domain::table_model::types::RepresentationInput as BorrowedRows;
use crate::domain::table_groups::types::Group;
use crate::domain::table_texts::types::Texts;
use ryo_derived::domain::key::types::On;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::Table;
use {
    crate::domain::table_groups::service::count_groups,
    crate::domain::table_groups::service::digest_groups,
    crate::domain::table_groups::types::CountRoom, crate::domain::table_groups::types::Hashes,
    ryo_values::domain::rows::service::push_row,
};
fn materialized(input: BorrowedRows<'_>) -> Table {
    let mut table = Table {
        kinds: input.kinds().to_vec(),
        ..Table::default()
    };
    let mut row = Vec::new();
    for at in 0..input.count().0 {
        if let Some(row) = input.read(RowIndex(at), &mut row) {
            push_row(&mut table, row);
        }
    }
    table
}
pub(super) fn groups(
    input: BorrowedRows<'_>,
    keys: &[ColumnIndex],
    distinct: Option<ColumnIndex>,
    digest: Option<ColumnIndex>,
    context: &mut OperatorContext<'_>,
    contract: &OperatorContract,
    admitted: &super::super::workspace::InputAdmissions<'_, '_>,
    semantics: &crate::application::file_dag::execution::PrimitiveSemanticsBridge<'_>,
) -> Result<Vec<Group>, OperatingError> {
    // The original collected contract checks the reduction operand before its grouping keys.
    if let Some(column) = digest.or(distinct) {
        crate::domain::table_model::service::check_columns(input.kinds(), &[column])?;
    }
    let use_ = prepared(
        contract,
        ryo_derived::domain::request::types::DerivedKind::PartitionOrder,
    )?;
    let acquired = admitted.acquire_rows(use_, context)?;
    let ryo_derived::domain::contract::types::DerivedView::PartitionOrder(order) =
        acquired.view()?
    else {
        return Err(OperatingError::WrongOperation);
    };
    let table = materialized(input);
    let mut groups = Vec::new();
    let mut values = Vec::new();
    let mut room = CountRoom {
        order,
        groups: &mut groups,
        values: &mut values,
    };
    if let Some(over) = digest {
        let texts = Texts {
            symbols: context.symbols.as_ref(),
            source: context.source,
            paths: context.paths,
            arenas: context.arenas,
        };
        digest_groups(
            &table,
            keys,
            over,
            &texts,
            &mut room,
            &mut Hashes::default(),
            semantics,
        )?;
    } else {
        count_groups(&table, On::Columns(keys), distinct, &mut room, semantics)?;
    }
    Ok(groups)
}

use ryo_values::domain::rows::types::RowIndex;
