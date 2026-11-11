//! Finite authored pattern slots, each compiled at most once, thread-safe: a source search's slot when the program is
//! compiled (the presence fold), every other slot when its operator first uses it.

use crate::domain::matching::types::PatternMatcher;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_texts::types::{RegexPattern, TextView};
use ryo_matching::inbound::use_cases::matching::{
    errors::MatchingError,
    types::{CompiledRegex, CompiledTextGlobs},
    types::{GlobPattern, RegexSource as AuthoredRegex},
};
use ryo_values::domain::plan_parts::types::{Operand, QueryLiteral, Test};
use ryo_values::domain::plans::types::Operation;
#[cfg(any(test, feature = "fixtures"))]
use ryo_values::domain::plans::types::Plan;
use std::sync::OnceLock;

/// Request-owned metadata only; runtime patterns cannot add keys or replace compiled slots.
#[derive(Debug, Default)]
pub struct PreparedMatchers {
    /// Exact test and literal bytes distinguish matcher families and glob polarity.
    slots: hashbrown::HashMap<Test, hashbrown::HashMap<RegexPattern, PreparedPattern>>,
    /// Actual first compilation events, absent from production.
    #[cfg(any(test, feature = "fixtures"))]
    pub builds: std::sync::atomic::AtomicUsize,
}

/// Compiled once: a source search's pattern by the presence fold, when the program is compiled; any other pattern when
/// the owning operator first reaches an applicable input row.
#[derive(Debug)]
enum PreparedPattern {
    /// Invalid expressions retain the same refusal, raised only at an actual match, replacement or search.
    Regex(OnceLock<Result<CompiledRegex, OperatingError>>),
    /// Unsupported glob compilation retains the interpreter outcome for subsequent invocations.
    Glob(OnceLock<CompiledTextGlobs>),
}

impl PreparedMatchers {
    /// Declare only authored literal keys from these plans, without interpreting their contents.
    #[cfg(any(test, feature = "fixtures"))]
    pub fn new(plans: &[Plan]) -> Self {
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
            #[expect(
                clippy::wildcard_enum_match_arm,
                reason = "only Select, Join and Axis carry literal comparisons and only Replace and SourceMatches a literal pattern; every other operation declares no literal key"
            )]
            let comparisons = match operation {
                Operation::Select { comparisons, .. } => comparisons.as_slice(),
                Operation::Join { filter, .. } => filter.as_slice(),
                Operation::Axis { domain_filter, .. } => domain_filter.as_slice(),
                Operation::Replace { pattern, .. } | Operation::SourceMatches { pattern, .. } => {
                    prepared.declare(
                        Test::Matches,
                        crate::domain::table_texts::types::TextView(&pattern.0),
                    );
                    continue;
                }
                _ => continue,
            };
            for comparison in comparisons {
                if let Operand::Literal(QueryLiteral::Text(pattern)) = &comparison.right {
                    prepared.declare(comparison.test, TextView(&pattern.0));
                }
            }
        }
        prepared
    }

    /// Preparation is the sole insertion boundary; parameters, functions and row cells never enter it.
    fn declare(&mut self, test: Test, pattern: TextView<'_>) {
        let slot = match test {
            Test::Matches => PreparedPattern::Regex(OnceLock::new()),
            Test::GlobAny | Test::NotGlobAny => PreparedPattern::Glob(OnceLock::new()),
            Test::Equal
            | Test::NotEqual
            | Test::LessThan
            | Test::LessThanOrEqual
            | Test::GreaterThan
            | Test::GreaterThanOrEqual
            | Test::FloatTextGreaterThanOrEqual
            | Test::In
            | Test::NotIn
            | Test::StartsWith
            | Test::EndsWith
            | Test::NumericGreaterThanOrEqual
            | Test::IntegerSuccessor
            | Test::NaturalLessThan => return,
        };
        let _slot = self
            .slots
            .entry(test)
            .or_default()
            .entry(RegexPattern(pattern.0.to_owned()))
            .or_insert(slot);
    }

    /// Read an exact authored regex slot after the evaluator has validated both text operands.
    pub fn regex(
        &self,
        pattern: TextView<'_>,
        matcher: PatternMatcher<'_>,
    ) -> Option<Result<&CompiledRegex, OperatingError>> {
        let PreparedPattern::Regex(slot) = self.slots.get(&Test::Matches)?.get(pattern.0)? else {
            return None;
        };
        Some(
            slot.get_or_init(|| {
                #[cfg(any(test, feature = "fixtures"))]
                let _previous = self
                    .builds
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                compile_regex(pattern, matcher)
            })
            .as_ref()
            .map_err(Clone::clone),
        )
    }

    /// Read an exact authored glob slot only after resolving an actual text subject.
    pub fn glob(
        &self,
        test: Test,
        pattern: TextView<'_>,
        matcher: PatternMatcher<'_>,
    ) -> Option<&CompiledTextGlobs> {
        let PreparedPattern::Glob(slot) = self.slots.get(&test)?.get(pattern.0)? else {
            return None;
        };
        Some(slot.get_or_init(|| {
            #[cfg(any(test, feature = "fixtures"))]
            let _previous = self
                .builds
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            matcher.0.compile_text_globs(&[GlobPattern(pattern.0)])
        }))
    }
}

/// Adapt semantic compilation refusal without changing the engine's cache or error meaning.
///
/// # Errors
///
/// [`OperatingError::InvalidRegex`] with the matcher's reason when the pattern does not compile.
pub fn compile_regex(
    pattern: TextView<'_>,
    matcher: PatternMatcher<'_>,
) -> Result<CompiledRegex, OperatingError> {
    matcher
        .0
        .compile_regex(AuthoredRegex(pattern.0))
        .map_err(|error| {
            let (MatchingError::InvalidRegex { reason } | MatchingError::BadGlob { reason, .. }) =
                error;
            OperatingError::InvalidRegex { reason }
        })
}
