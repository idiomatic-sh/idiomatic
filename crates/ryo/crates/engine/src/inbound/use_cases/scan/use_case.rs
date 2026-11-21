//! The `scan` use case: the worker's state reset for the file, then every rule dispatched: each branch whose need holds runs
//! through the one loop, over the fill side's store.

use super::errors::ScanError;
use super::interface::ScanUseCase;
use crate::domain::dispatch::service::dispatch;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::filled::errors::FilledError;
use crate::domain::findings::types::{FoundRows, ScanCounters};
use crate::domain::made_tables::types::FillFrame;
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::service::rule_set;
use crate::domain::programs::types::Programs;
use crate::domain::scratch::service::next_file;
use crate::domain::scratch::types::{FileFault, ScanScratch, ScratchEngine};
use ryo_language::domain::facts::types::FileFacts;
use ryo_operators::domain::patterns::types::{PatternMatcher, PatternSemantics};
use ryo_values::domain::stored_facts::types::PartialCause;
use std::sync::Arc;

/// Runs rule sets' programs on files. It holds the matching semantics its composer hands it: a text test of a rule (a
/// regular expression, a glob) is matched through them, and the engine builds none.
///
/// # Examples
///
/// ```
/// use ryo_engine::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
/// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
/// use std::sync::Arc;
///
/// let _use_case = ScanUseCaseImpl::new(Arc::new(MatchingUseCaseImpl));
/// ```
#[derive(Clone)]
pub struct ScanUseCaseImpl {
    /// The matching semantics.
    matcher: Arc<dyn PatternSemantics + Send + Sync>,
}

impl std::fmt::Debug for ScanUseCaseImpl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}

impl ScanUseCaseImpl {
    /// The use case over the matching semantics `matcher`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_engine::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
    /// use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
    /// use std::sync::Arc;
    ///
    /// let _use_case = ScanUseCaseImpl::new(Arc::new(MatchingUseCaseImpl));
    /// ```
    #[must_use]
    pub fn new(matcher: Arc<dyn PatternSemantics + Send + Sync>) -> Self {
        Self { matcher }
    }
}

impl ScanUseCase for ScanUseCaseImpl {
    fn scan(
        &self,
        programs: &Programs,
        file: FileFacts<'_>,
        scratch: &mut ScanScratch,
        found: &mut dyn FoundRows,
    ) -> Result<ScanCounters, ScanError> {
        next_file(scratch);
        let ScratchEngine {
            state,
            store,
            words,
        } = &mut scratch.0;
        let mut fill = FillFrame {
            programs: rule_set(&programs.0),
            nodes: file.table,
            file: file.file,
            path: file.path,
            text: file.source,
            facts: file.facts,
            words,
            store,
            matcher: PatternMatcher(&*self.matcher),
        };
        let counters = dispatch(
            &mut StepFrame {
                fill: &mut fill,
                state: &mut *state,
                found,
                ports: &TheLoop,
            },
            programs,
        );
        // The load refuses a rule that can reach a word or a fill the executor does not run: none reaches a scan.
        debug_assert_eq!(
            state.work.unbuilt.0, 0,
            "a word or a fill the executor does not run reached the scan"
        );
        match state.fault.take() {
            None => Ok(counters),
            Some(FileFault { rule, fault }) => Err(match fault {
                FilledError::Refused { relation, cause } => ScanError::Refused {
                    rule,
                    relation,
                    cause,
                },
                FilledError::Partial { relation, rows } => ScanError::Partial {
                    rule,
                    relation,
                    cause: PartialCause(rows.cause.0),
                    missing: rows.missing,
                },
                // A fill the engine does not run and a number the code does not hold are not faults of a file: the loop
                // counts the first and keeps neither as the file's fault.
                FilledError::Unbuilt { .. } | FilledError::Code { .. } => return Ok(counters),
                // A per-row meaning's fault on a row is the rule's fault in the file, and `ScanError` has no variant to
                // say it yet (the fill side's QUESTIONS.md Q4): until it has, the scan answers its counters.
                FilledError::Meaning { .. } => return Ok(counters),
            }),
        }
    }
}
