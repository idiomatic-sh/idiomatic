//! Finite authored pattern slots with demand-driven thread-safe compilation.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_texts::types::RegexPattern;
use ryo_matching::inbound::use_cases::matching::{
    compiled::{CompiledRegex, CompiledTextGlobs},
    errors::MatchingError,
    interface::MatchingUseCase,
    types::{GlobPattern, RegexSource as AuthoredRegex},
    use_case::MatchingUseCaseImpl,
};
use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral, Test};
use ryo_values::domain::plans::types::Operation;
#[cfg(test)]
use ryo_values::domain::plans::types::Plan;
use std::sync::OnceLock;

/// Request-owned metadata only; runtime patterns cannot add keys or replace compiled slots.
#[derive(Debug, Default)]
pub(crate) struct PreparedMatchers {
    /// Exact test and literal bytes distinguish matcher families and glob polarity.
    slots: hashbrown::HashMap<Test, hashbrown::HashMap<RegexPattern, PreparedPattern>>,
    /// Actual first compilation events, absent from production.
    #[cfg(test)]
    pub builds: std::sync::atomic::AtomicUsize,
}

/// Compilation is deferred until the owning operator reaches an applicable input row.
#[derive(Debug)]
enum PreparedPattern {
    /// Invalid expressions retain the same refusal, raised only at an actual match/replacement.
    Regex(OnceLock<Result<CompiledRegex, OperatingError>>),
    /// Unsupported glob compilation retains the interpreter outcome for subsequent invocations.
    Glob(OnceLock<CompiledTextGlobs>),
}

impl PreparedMatchers {
    /// Declare only authored literal keys from these plans, without interpreting their contents.
    #[cfg(test)]
    pub(crate) fn new(plans: &[Plan]) -> Self {
        Self::from_operations(
            plans
                .iter()
                .flat_map(|plan| plan.steps.iter().map(|step| &step.operation)),
        )
    }

    /// Declare literal keys from the one compiled executable program, before any worker runs.
    pub(crate) fn from_operations<'a>(operations: impl IntoIterator<Item = &'a Operation>) -> Self {
        let mut prepared = Self::default();
        for operation in operations {
            let comparisons = match operation {
                Operation::Select { comparisons, .. } => comparisons.as_slice(),
                Operation::Join { filter, .. } => filter.as_slice(),
                Operation::Axis { domain_filter, .. } => domain_filter.as_slice(),
                Operation::Replace { pattern, .. } | Operation::SourceMatches { pattern, .. } => {
                    prepared.declare(Test::Matches, &pattern.0);
                    continue;
                }
                _ => continue,
            };
            for comparison in comparisons {
                if let Operand::Literal(QueryLiteral::Text(pattern)) = &comparison.right {
                    prepared.declare(comparison.test, &pattern.0);
                }
            }
        }
        prepared
    }

    /// Preparation is the sole insertion boundary; parameters, functions and row cells never enter it.
    fn declare(&mut self, test: Test, pattern: &str) {
        let slot = match test {
            Test::Matches => PreparedPattern::Regex(OnceLock::new()),
            Test::GlobAny | Test::NotGlobAny => PreparedPattern::Glob(OnceLock::new()),
            _ => return,
        };
        let _slot = self
            .slots
            .entry(test)
            .or_default()
            .entry(RegexPattern(pattern.to_owned()))
            .or_insert(slot);
    }

    /// Read an exact authored regex slot after the evaluator has validated both text operands.
    pub(crate) fn regex(&self, pattern: &str) -> Option<Result<&CompiledRegex, OperatingError>> {
        let PreparedPattern::Regex(slot) = self.slots.get(&Test::Matches)?.get(pattern)? else {
            return None;
        };
        Some(
            slot.get_or_init(|| {
                #[cfg(test)]
                let _previous = self
                    .builds
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                compile_regex(pattern)
            })
            .as_ref()
            .map_err(Clone::clone),
        )
    }

    /// Read an exact authored glob slot only after resolving an actual text subject.
    pub(crate) fn glob(&self, test: Test, pattern: &str) -> Option<&CompiledTextGlobs> {
        let PreparedPattern::Glob(slot) = self.slots.get(&test)?.get(pattern)? else {
            return None;
        };
        Some(slot.get_or_init(|| {
            #[cfg(test)]
            let _previous = self
                .builds
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            MatchingUseCaseImpl::new().compile_text_globs(&[GlobPattern(pattern)])
        }))
    }
}

/// Adapt semantic compilation refusal without changing the engine's cache or error meaning.
pub(crate) fn compile_regex(pattern: &str) -> Result<CompiledRegex, OperatingError> {
    MatchingUseCaseImpl::new()
        .compile_regex(AuthoredRegex(pattern))
        .map_err(|error| {
            let (MatchingError::InvalidRegex { reason } | MatchingError::BadGlob { reason, .. }) =
                error;
            OperatingError::InvalidRegex { reason }
        })
}
