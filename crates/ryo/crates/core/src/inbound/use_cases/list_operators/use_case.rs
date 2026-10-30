//! The `list_operators` use case: it asks the operator names for each operator's description.

use super::errors::ListOperatorsError;
use super::interface::ListOperatorsUseCase;
use super::types::{ListOperatorsReport, OperatorSummary};
use crate::domain::operator::constants::{OPERATORS, describe};

/// `list_operators`; the operators are the domain's own, so it calls no port.
#[derive(Clone, Copy, Debug, Default)]
pub struct ListOperatorsUseCaseImpl;

impl ListOperatorsUseCaseImpl {
    /// `list_operators`, which calls no port.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl ListOperatorsUseCase for ListOperatorsUseCaseImpl {
    fn list_operators(&self) -> Result<ListOperatorsReport, ListOperatorsError> {
        let operators = OPERATORS
            .into_iter()
            .map(describe)
            .map(|description| OperatorSummary {
                name: description.name,
                summary: description.summary,
            })
            .collect();
        Ok(ListOperatorsReport { operators })
    }
}
