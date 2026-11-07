//! State owned by a bounded file analysis invocation.
use super::errors::FileAnalysisError;
use crate::domain::workers::types::Tables;
use ryo_execution::outbound::facts::interface::FactsWorker;
use ryo_execution::application::file_dag::program::prepared_execution::types::PreparedExecutionProgram;
use ryo_execution::application::file_dag::program::request_execution::types::RequestExecution;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use super::constants::{FILE, NODES, SEMANTICSTATE, THE_SELECTED_LANGUAGE_DOES_NOT_OFFER};
use ryo_execution::inbound::use_cases::execution::interface::{OperatingError, SemanticLookup};
use ryo_language::domain::elements::service::real_count;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::semantics::service::admit;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest, SemanticSession};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, FileId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
/// Whether the rule results of one file generation are all there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileCompleteness {
    /// Every rule result is whole.
    Complete,
    /// A rule result is missing, so no warm or edit record may be made of the file.
    Incomplete,
}

/// Whether a rule runs on one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleGate {
    /// The rule runs.
    Open,
    /// The rule does not run: its applicability is not met or not known.
    Blocked,
}

/// What one worker owns while the jobs run: its tables, its handle of the fact store and the first thing that went wrong in it.
pub(crate) struct Worker {
    /// What the worker owns besides its handle.
    pub tables: Tables,
    /// Where the rows of the file being read and the words kept of it are.
    pub facts: Box<dyn FactsWorker>,
    /// One bounded persisted-frontier reader for this worker's increasing jobs.
    pub old_rows: Option<Box<dyn crate::outbound::lint_store::interface::OldRowBatch>>,
    /// The failure that stopped the worker, when one did.
    pub failure: Option<FileAnalysisError>,
}

/// What a visit of a file reads that is the same for every file, and the ports a visit reaches besides.
pub(crate) struct FileVisit<'a> {
    /// One run's snapshot owns temporary normalized spans beyond any pre-read worker lifetime.
    pub store: Option<&'a dyn crate::outbound::lint_store::interface::ReadStore>,
    /// What the domain reads.
    pub setting: RequestVisitSetting<'a>,
}

use crate::domain::visit::types::{Noting, RoleRead};
use crate::outbound::language::types::{NoteWriting, RoleWriting};
impl From<Noting> for NoteWriting {
    fn from(noting: Noting) -> Self {
        match noting {
            Noting::Wanted => Self::Wanted,
            Noting::Unwanted => Self::Unwanted,
        }
    }
}
impl From<RoleRead> for RoleWriting {
    fn from(read: RoleRead) -> Self {
        match read {
            RoleRead::Reads => Self::Wanted,
            RoleRead::Skips => Self::Unwanted,
        }
    }
}

/// What a visit reads that the domain reads, with the request's provider, program and execution capabilities.
#[derive(Clone)]
pub(crate) struct RequestVisitSetting<'a> {
    /// Explicit request provider; none preserves ordinary path recognition.
    pub source_language: Option<&'a ryo_values::domain::descriptions::types::LanguageName>,
    /// One prepared executable program shared by all scopes in this request.
    pub program: Arc<PreparedExecutionProgram>,
    /// Required application capabilities; no domain code owns or constructs them.
    pub services: &'a RequestExecution,
    /// What the domain reads of every file.
    pub domain: crate::domain::workers::types::VisitSetting<'a>,
}

impl<'a> Deref for RequestVisitSetting<'a> {
    type Target = crate::domain::workers::types::VisitSetting<'a>;
    fn deref(&self) -> &Self::Target {
        &self.domain
    }
}

impl DerefMut for RequestVisitSetting<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.domain
    }
}

/// The application keeps this only while evaluating one freshly parsed file.
pub(crate) struct SemanticState {
    file: FileId,
    nodes: Count,
    session: RefCell<Option<Box<dyn SemanticSession>>>,
    completeness: Cell<FileCompleteness>,
}

impl std::fmt::Debug for SemanticState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(SEMANTICSTATE)
            .field(FILE, &self.file)
            .field(NODES, &self.nodes)
            .finish_non_exhaustive()
    }
}

impl SemanticState {
    /// Own the selected provider's session and this file generation's exact node bound.
    pub(crate) fn new(
        file: FileId,
        elements: &Elements,
        session: Option<Box<dyn SemanticSession>>,
    ) -> Self {
        Self {
            file,
            nodes: real_count(elements),
            session: RefCell::new(session),
            completeness: Cell::new(FileCompleteness::Complete),
        }
    }

    /// An incomplete rule result cannot become a complete warm/edit record.
    pub(crate) fn mark_incomplete(&self) {
        self.completeness.set(FileCompleteness::Incomplete);
    }

    /// Whether every rule result of this file generation is whole.
    pub(crate) fn completeness(&self) -> FileCompleteness {
        self.completeness.get()
    }
}

impl SemanticLookup for SemanticState {
    fn render_batch(
        &self,
        requests: &[RenderRequest],
        source: ryo_values::domain::files::types::SourceView<'_>,
    ) -> Result<Vec<RenderOutcome>, OperatingError> {
        // Deduplicate only this invocation. Retaining answers across invocations
        // requires admission through the common owner and is not done here.
        let mut unique = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for request in requests {
            admit(request, self.file, self.nodes)?;
            if seen.insert(*request) {
                unique.push(*request);
            }
        }
        let mut answers_by_key = HashMap::new();
        if !unique.is_empty() {
            let mut session = self.session.borrow_mut();
            let Some(session) = session.as_deref_mut() else {
                return Err(OperatingError::SemanticIncomplete {
                    reason: THE_SELECTED_LANGUAGE_DOES_NOT_OFFER.into(),
                });
            };
            let answers = session.render_batch(ByteSlice(source.0), &unique)?;
            if answers.len() != unique.len() {
                return Err(OperatingError::SemanticBatchShape);
            }
            for (request, answer) in unique.into_iter().zip(answers) {
                answers_by_key.insert(request, answer);
            }
        }
        let mut ordered = Vec::with_capacity(requests.len());
        for request in requests {
            ordered.push(
                answers_by_key
                    .get(request)
                    .ok_or(OperatingError::SemanticBatchShape)?
                    .clone(),
            );
        }
        Ok(ordered)
    }
}
