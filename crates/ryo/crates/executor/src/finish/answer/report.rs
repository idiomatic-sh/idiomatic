//! Typed report payload, independent of how many consumers admit it.
use super::Row;
use std::ops::Deref;

#[derive(Clone, Default)]
pub struct Reported {
    pub rows: Box<[Row]>,
    pub issues: Option<Box<ReportIssues>>,
}

#[derive(Clone, Default)]
pub struct ReportIssues {
    pub rejected_syntax: Vec<String>,
    pub unknowns: Vec<(String, String)>,
}

impl Deref for Reported {
    type Target = ReportIssues;

    fn deref(&self) -> &ReportIssues {
        static EMPTY: ReportIssues = ReportIssues {
            rejected_syntax: Vec::new(),
            unknowns: Vec::new(),
        };
        self.issues.as_deref().unwrap_or(&EMPTY)
    }
}

impl Reported {
    pub fn new(
        rows: Vec<Row>,
        unknowns: Vec<(String, String)>,
        rejected_syntax: Vec<String>,
    ) -> Self {
        let issues = (!unknowns.is_empty() || !rejected_syntax.is_empty())
            .then(|| Box::new(ReportIssues { unknowns, rejected_syntax }));
        Self { rows: rows.into_boxed_slice(), issues }
    }

    pub fn skipped(paths: &[String]) -> Self {
        use ryo_execution_domain::domain::operating::errors::OperatingError;
        let error = OperatingError::ObservationIncomplete {
            reason: "every source partition was explicitly skipped".into(),
        };
        let reason = format!("project phase diagnostic emit: {error}");
        let unknowns = paths.iter().map(|path| (path.clone(), reason.clone())).collect();
        Self::new(Vec::new(), unknowns, Vec::new())
    }

    pub fn extend_unknowns(&mut self, unknowns: Vec<(String, String)>) {
        if !unknowns.is_empty() {
            self.issues.get_or_insert_with(Default::default).unknowns.extend(unknowns);
        }
    }
}

impl From<Vec<Row>> for Reported {
    fn from(rows: Vec<Row>) -> Self {
        Self::new(rows, Vec::new(), Vec::new())
    }
}
