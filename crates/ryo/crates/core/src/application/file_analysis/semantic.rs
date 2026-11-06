//! One file generation's semantic demand and provider session.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use ryo_execution::inbound::use_cases::execution::types::OperatingError;
use ryo_execution::inbound::use_cases::execution::types::SemanticLookup;
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
    nodes: usize,
    session: RefCell<Option<Box<dyn SemanticSession>>>,
    incomplete: Cell<bool>,
}

impl std::fmt::Debug for SemanticState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticState")
            .field("file", &self.file)
            .field("nodes", &self.nodes)
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
            nodes: real_count(elements).0,
            session: RefCell::new(session),
            incomplete: Cell::new(false),
        }
    }

    /// An incomplete rule result cannot become a complete warm/edit record.
    pub(crate) fn mark_incomplete(&self) {
        self.incomplete.set(true);
    }

    pub(crate) fn is_incomplete(&self) -> bool {
        self.incomplete.get()
    }
}

impl SemanticLookup for SemanticState {
    fn render_batch(
        &self,
        requests: &[RenderRequest],
        source: &[u8],
    ) -> Result<Vec<RenderOutcome>, OperatingError> {
        // Deduplicate only this invocation. Retaining answers across invocations
        // requires admission through the common owner and is not done here.
        let mut unique = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for request in requests {
            admit(request, self.file, Count(self.nodes))?;
            if seen.insert(*request) {
                unique.push(*request);
            }
        }
        let mut answers_by_key = HashMap::new();
        if !unique.is_empty() {
            let mut session = self.session.borrow_mut();
            let Some(session) = session.as_deref_mut() else {
                return Err(OperatingError::SemanticIncomplete {
                    reason: "the selected language does not offer semantic rendering".into(),
                });
            };
            let answers = session.render_batch(ByteSlice(source), &unique)?;
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
