//! The `evaluate` use case: the replacement candidate of the interpreter's `run_file`.

use super::types::EvaluateInput;
use crate::outbound::facts::interface::Facts;
use ryo_execution::inbound::use_cases::execution::interface::FileRules;
use ryo_language::domain::symbols::types::Symbols;

/// Evaluates the rules of a request on one file at a time.
pub trait EvaluateUseCase {
    /// The rules the file runs, in the program's rule order, each with its emitted root, as `run_file` answers them: every root
    /// select is served by one walk of `facts`, and each other relation is asked of `facts` only when a rule still has rows to
    /// read it with. Names the evaluation makes are interned in `symbols`, the file's names.
    fn evaluate(
        &self,
        input: EvaluateInput<'_>,
        symbols: &mut Symbols,
        facts: &dyn Facts,
    ) -> FileRules;
}
