//! The `describe` use case: it asks the syntax port for the grammar it reads.

use std::sync::Arc;

use super::errors::DescribeError;
use super::interface::DescribeUseCase;
use super::types::DescribeReport;
use crate::outbound::syntax::interface::SyntaxPort;

/// `describe`, over the syntax it asks.
pub struct DescribeUseCaseImpl {
    /// Answers the grammar it reads programs by.
    syntax: Arc<dyn SyntaxPort>,
}

impl std::fmt::Debug for DescribeUseCaseImpl {
    /// The use case for a debug print: its port is a trait object with no `Debug`, so it names only itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl DescribeUseCaseImpl {
    /// `describe` over the port the edge injects.
    #[must_use]
    pub fn new(syntax: Arc<dyn SyntaxPort>) -> Self {
        Self { syntax }
    }
}

impl DescribeUseCase for DescribeUseCaseImpl {
    fn describe(&self) -> Result<DescribeReport, DescribeError> {
        Ok(DescribeReport {
            grammar: self.syntax.grammar(),
        })
    }
}
