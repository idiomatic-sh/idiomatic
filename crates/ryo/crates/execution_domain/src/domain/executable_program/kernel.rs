//! Semantic kernels have only admitted borrowed inputs, views and scratch.
//! They cannot ask an engine to resolve a dependency or mint a publication.
/// The first applicable row gate precedes invocation resource acquisition. It
/// reads admitted values/text only and has no application acquisition capability.
pub(crate) fn row_admissible(
    contract: &crate::domain::operator::contract::types::OperatorContract,
    row: &[ryo_relations::domain::relations::types::Value],
    texts: &crate::domain::table_texts::types::Texts<'_>,
) -> Result<
    crate::domain::executable_program::types::RowGate,
    crate::domain::operating::errors::OperatingError,
> {
    use {
        crate::domain::operator::contract::types::SourceGate,
        crate::domain::executable_program::types::RowGate,
        crate::domain::table_texts::service::datum, crate::domain::table_texts::types::Datum,
        ryo_values::domain::rows::service::cell,
    };
    if let Some(error) = &contract.row_error {
        return Err(error.clone());
    }
    match contract.source_gate {
        SourceGate::None => Ok(RowGate::Admitted),
        SourceGate::CheckedSpan { span, base_line } => Ok(
            match crate::domain::operator::contract::views::validate(
                row,
                span,
                base_line,
                texts.source,
            )? {
                Some(_) => RowGate::Admitted,
                None => RowGate::Withheld,
            },
        ),
        SourceGate::CheckedText { column } => {
            if matches!(datum(cell(row, column)?, texts), Datum::Text(_)) {
                Ok(RowGate::Admitted)
            } else {
                Err(crate::domain::operating::errors::OperatingError::ShapeMismatch)
            }
        }
    }
}
