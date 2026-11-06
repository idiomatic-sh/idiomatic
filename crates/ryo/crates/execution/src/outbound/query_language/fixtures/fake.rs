//! The fake of the `query_language` port: what the core's tests stand in for it with.
use super::super::errors::QueryLanguageError;
use super::super::interface::QueryLanguagePort;
use ryo_values::domain::{
    descriptions::types::ValidateReport, hosted::types::HostedRelation, plans::types::Plan,
    program::types::QueryBundle,
};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Query compiler injection only; the cache and binding owner remain the real service.
#[derive(Debug)]
pub struct QueryLanguageFake {
    /// The plan returned by successful compile calls.
    pub plan: Plan,
    /// Observed compile calls, including refusals.
    pub calls: AtomicUsize,
}
/// Text marker which the fixture compiler refuses.
pub const REFUSED: &str = "refuse";
impl Default for QueryLanguageFake {
    fn default() -> Self {
        use ryo_values::domain::{
            plan_parts::types::{ColumnIndex, StepId},
            plans::types::Emit,
        };
        Self {
            plan: Plan {
                observation_inputs: Vec::new(),
                observations: Vec::new(),
                parameters: Vec::new(),
                steps: Vec::new(),
                emit: Emit {
                    step: StepId(0),
                    subject: ColumnIndex(0),
                    key: Vec::new(),
                    headings: Vec::new(),
                },
                frontier: Vec::new(),
                comparison_origins: Vec::new(),
            },
            calls: AtomicUsize::new(0),
        }
    }
}
impl QueryLanguagePort for QueryLanguageFake {
    fn validate(
        &self,
        bundle: &QueryBundle,
        _hosted: &[HostedRelation],
        _observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError> {
        if bundle.root.text.contains(REFUSED) {
            return Err(compiler_refusal());
        }
        Ok(ValidateReport {
            relations: Vec::new(),
            operators: Vec::new(),
            steps: ryo_values::domain::descriptions::types::StepCount(self.plan.steps.len()),
        })
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        _hosted: &[HostedRelation],
        _observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        let _previous = self.calls.fetch_add(1, Ordering::SeqCst);
        if bundle.root.text.contains(REFUSED) {
            return Err(compiler_refusal());
        }
        Ok(self.plan.clone())
    }
}
fn compiler_refusal() -> QueryLanguageError {
    use ryo_relations::domain::relations::types::{Offset, Span};
    use ryo_values::domain::program::types::RefusalMessage;
    use ryo_values::domain::program::types::RefusedKind;
    use ryo_values::domain::program::types::RefusedQuery;
    QueryLanguageError::Refused {
        error: RefusedQuery {
            kind: RefusedKind::Invalid,
            message: RefusalMessage("the compiler provider refuses this query".into()),
            span: Span {
                start: Offset(0),
                end: Offset(0),
            },
            origin: None,
        },
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
