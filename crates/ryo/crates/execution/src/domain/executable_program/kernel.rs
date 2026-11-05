//! Semantic kernels have only admitted borrowed inputs, views and scratch.
//! They cannot ask an engine to resolve a dependency or mint a publication.
/// The first applicable row gate precedes invocation resource acquisition. It
/// reads admitted values/text only and has no application acquisition capability.
pub(crate) fn row_admissible(
    contract: &crate::domain::operator::contract::types::OperatorContract,
    row: &[ryo_relations::domain::relations::types::Value],
    texts: &crate::domain::table_texts::types::Texts<'_>,
) -> Result<bool, crate::domain::operating::errors::OperatingError> {
    use crate::domain::{
        operator::contract::types::SourceGate,
        table_model::service::cell,
        table_texts::{service::datum, types::Datum},
    };
    if let Some(error) = &contract.row_error {
        return Err(error.clone());
    }
    match contract.source_gate {
        SourceGate::None => Ok(true),
        SourceGate::CheckedSpan { span, base_line } => Ok(
            crate::domain::operator::collections::operators::source_matches::source::validate(
                row,
                span,
                base_line,
                texts.source.0,
            )?
            .is_some(),
        ),
        SourceGate::CheckedText { column } => {
            if matches!(datum(cell(row, column)?, texts), Datum::Text(_)) {
                Ok(true)
            } else {
                Err(crate::domain::operating::errors::OperatingError::ShapeMismatch)
            }
        }
    }
}
