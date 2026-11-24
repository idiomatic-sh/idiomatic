//! Report payloads survive until the last admitted consumer finishes.
use super::Row;
use std::{ops::Deref, sync::Arc};

/// An empty successful answer is distinct from a refused answer. It has no
/// allocation and takes up one word in the answer slot.
#[derive(Clone, Default)]
pub(crate) struct Reported(Option<Arc<ReportData>>);

#[derive(Clone, Default)]
pub(crate) struct ReportData {
    pub(crate) rows: Vec<Row>,
    pub(crate) rejected_syntax: Vec<String>,
    pub(crate) unknowns: Vec<(String, String)>,
}

impl Deref for Reported {
    type Target = ReportData;

    fn deref(&self) -> &ReportData {
        static EMPTY: ReportData = ReportData {
            rows: Vec::new(),
            rejected_syntax: Vec::new(),
            unknowns: Vec::new(),
        };
        self.0.as_deref().unwrap_or(&EMPTY)
    }
}

impl Reported {
    pub(crate) fn new(
        rows: Vec<Row>,
        unknowns: Vec<(String, String)>,
        rejected_syntax: Vec<String>,
    ) -> Self {
        if rows.is_empty() && unknowns.is_empty() && rejected_syntax.is_empty() {
            return Self::default();
        }
        Self(Some(Arc::new(ReportData {
            rows,
            rejected_syntax,
            unknowns,
        })))
    }

    pub(crate) fn skipped(paths: &[String]) -> Self {
        use ryo_execution_domain::domain::operating::errors::OperatingError;
        let error = OperatingError::ObservationIncomplete {
            reason: "every source partition was explicitly skipped".into(),
        };
        let reason = format!("project phase diagnostic emit: {error}");
        let unknowns = paths.iter().map(|path| (path.clone(), reason.clone())).collect();
        Self::new(Vec::new(), unknowns, Vec::new())
    }

    pub(crate) fn extend_unknowns(&mut self, unknowns: Vec<(String, String)>) {
        if !unknowns.is_empty() {
            let body = self.0.get_or_insert_with(Default::default);
            Arc::make_mut(body).unknowns.extend(unknowns);
        }
    }
}

impl From<Vec<Row>> for Reported {
    fn from(rows: Vec<Row>) -> Self {
        Self::new(rows, Vec::new(), Vec::new())
    }
}
