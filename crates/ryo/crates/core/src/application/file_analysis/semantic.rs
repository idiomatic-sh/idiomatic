//! One file generation's semantic demand and provider session.

use crate::application::file_analysis::constants::{
    FILE, NODES, SEMANTICSTATE, THE_SELECTED_LANGUAGE_DOES_NOT_OFFER,
};
use super::types::FileCompleteness;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use ryo_execution::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution::inbound::use_cases::execution::interface::SemanticLookup;
use ryo_language::domain::elements::service::real_count;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::semantics::types::SemanticSession;
use ryo_language::domain::semantics::service::admit;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::{Count, FileId};

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
