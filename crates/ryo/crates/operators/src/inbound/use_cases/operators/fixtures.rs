//! The test support of the `operators` use case: the capability and the matcher its tests share.
use super::interface::OperatorsUseCase;
use super::types::{OperatorAnswer, OperatorCall};
use super::use_case::OperatorsUseCaseImpl;
use crate::domain::contract::errors::ContractError;
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::source_matches::types::TextRowDriver;
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase,
    types::{CompiledRegex, RegexSource},
    use_case::MatchingUseCaseImpl,
};
use ryo_relations::domain::relations::types::{Integer, Offset, Span, Value as Cell};
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::row_texts::types::CellText;

/// The operators capability composed as an edge composes it.
#[must_use]
pub fn operators() -> impl OperatorsUseCase {
    OperatorsUseCaseImpl::new(std::sync::Arc::new(
        crate::outbound::matching::factory::use_case(),
    ))
}

/// A compiled regex of an authored pattern.
///
/// # Errors
///
/// [`MatchingError::InvalidRegex`](ryo_matching::inbound::use_cases::matching::errors::MatchingError::InvalidRegex) when
/// the pattern is not valid regex syntax.
pub fn regex(
    pattern: &str,
) -> Result<CompiledRegex, ryo_matching::inbound::use_cases::matching::errors::MatchingError> {
    MatchingUseCaseImpl::new().compile_regex(RegexSource(pattern))
}

/// A number cell.
#[must_use]
pub fn number(value: i64) -> Cell {
    Cell::Number(Integer(value))
}

/// A span cell between two offsets.
#[must_use]
pub fn span(start: u32, end: u32) -> Cell {
    Cell::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

/// Runs one call and asserts that it wrote its output.
///
/// # Errors
/// Returns the operator fault the call answers.
///
/// # Panics
/// Panics when the call answers anything but written.
pub fn written(call: OperatorCall<'_>) -> Result<(), PortableOperatorFault> {
    assert_eq!(operators().operators(call)?, OperatorAnswer::Written);
    Ok(())
}

/// The delivery of source matches that records what reached it and can refuse its second row.
#[derive(Default)]
pub struct SourceDelivery {
    /// The rows delivered.
    pub rows: Vec<Vec<Cell>>,
    /// How many deliveries were attempted.
    pub attempts: usize,
    /// How many texts were interned.
    pub interns: u32,
    /// The interned texts in order.
    pub names: Vec<String>,
    /// Whether the second delivery is refused.
    pub fail_second: bool,
}

impl TextRowDriver for SourceDelivery {
    fn intern(&mut self, text: CellText<'_>) -> Cell {
        self.interns += 1;
        assert!(
            matches!(text.0, "b" | "c" | "d"),
            "capture resolves the original physical text"
        );
        self.names.push(text.0.to_owned());
        Cell::Symbol(Symbol(self.interns))
    }
    fn emit(&mut self, row: &[Cell]) -> Result<(), ContractError> {
        self.attempts += 1;
        if self.fail_second && self.attempts == 2 {
            return Err(ContractError::Interrupted);
        }
        self.rows.push(row.to_vec());
        Ok(())
    }
}
