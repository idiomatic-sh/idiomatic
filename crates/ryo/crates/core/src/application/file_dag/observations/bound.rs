//! Decode owned producer words in the consuming arena, then use the common distinct Union once.
use super::union;
use crate::application::file_dag::executor::ScopedExecutor;
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::types::OperatorContract,
    table_model::{service::materialize, types::Table},
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::stored_facts::types::KeptWords;
use std::sync::Arc;

/// Decode each actual producer under the caller's Symbols before constructing its operator context.
pub(crate) fn decoded(
    words: &[Arc<KeptWords>],
    symbols: &mut Symbols,
) -> Result<Vec<Table>, OperatingError> {
    words
        .iter()
        .map(|words| {
            crate::domain::table_words::service::decode_complete(
                &words.0,
                ryo_language::domain::symbols::service::places_mut(symbols),
            )
            .map_err(|error| OperatingError::ObservationIncomplete {
                reason: error.to_string(),
            })
        })
        .collect()
}
/// The same registered Union and common owner serve zero, singleton and many producers.
/// There is no observation-specific set algorithm or producer execution here.
pub(crate) fn combine(
    mut tables: Vec<Table>,
    kinds: &[CellKind],
    contract: &OperatorContract,
    executor: &ScopedExecutor<'_>,
    context: &mut OperatorContext<'_>,
) -> Result<Table, OperatingError> {
    for table in &mut tables {
        if table.count.0 == 0 && table.kinds.is_empty() {
            crate::domain::table_model::service::reset(table, kinds);
        }
    }
    let owner = std::rc::Rc::clone(executor.owner());
    let sources = tables
        .into_iter()
        .map(|table| owner.computed(table))
        .collect::<Result<Vec<_>, _>>()?;
    let read = |_| Err(OperatingError::NotRead);
    let result = union::combine(&sources, kinds, contract, &read, executor, context)?;
    let rows = owner.resolve(result.as_ref(), &read)?;
    let mut table = Table::default();
    materialize(&rows, &mut table, &mut Vec::new());
    Ok(table)
}
