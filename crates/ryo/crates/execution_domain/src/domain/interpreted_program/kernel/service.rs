//! The `kernel` service: free functions named by verb over the domain's types, with no IO and no ports.

/// The first applicable row gate precedes invocation resource acquisition. It
/// reads admitted values/text only and has no application acquisition capability.
///
/// # Errors
///
/// The contract's recorded row error when its preparation kept one; `ShapeMismatch` when a checked text column
/// holds no text; the error checking the row's source span or reading its cell returns.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::interpreted_program::kernel::service::row_admissible;
/// use ryo_execution_domain::domain::interpreted_program::types::RowGate;
/// use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
/// use ryo_operators::domain::table_texts::types::Texts;
/// use ryo_relations::domain::relations::types::Value;
/// fn gate(contract: &OperatorContract, row: &[Value], texts: &Texts<'_>) -> Option<RowGate> {
///     row_admissible(contract, row, texts).ok()
/// }
/// ```
pub fn row_admissible(
    contract: &crate::domain::operator::contract::types::OperatorContract,
    row: &[ryo_relations::domain::relations::types::Value],
    texts: &ryo_operators::domain::table_texts::types::Texts<'_>,
) -> Result<
    crate::domain::interpreted_program::types::RowGate,
    crate::domain::operating::errors::OperatingError,
> {
    use {
        crate::domain::interpreted_program::types::RowGate,
        crate::domain::operator::contract::types::SourceGate,
        ryo_operators::domain::table_texts::service::datum,
        ryo_operators::domain::table_texts::types::Datum, ryo_values::domain::rows::service::cell,
    };
    if let Some(error) = &contract.row_error {
        return Err(error.clone());
    }
    match contract.source_gate {
        SourceGate::None => Ok(RowGate::Admitted),
        SourceGate::CheckedSpan { span, base_line } => Ok(
            match crate::domain::operator::contract::views::service::validate(
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
