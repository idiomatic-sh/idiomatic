//! Exact multisets preserve duplicated findings.
use crate::domain::diagnostics::types::Difference;
use crate::domain::diagnostics::types::RunReport;

/// Compare canonical finding multisets and exact submitted-file coverage.
pub fn compare(reference: &RunReport, candidate: &RunReport) -> Result<(), Difference> {
    let mut left = reference.diagnostics.clone();
    let mut right = candidate.diagnostics.clone();
    left.sort();
    right.sort();
    let mut left_files = reference.files.clone();
    let mut right_files = candidate.files.clone();
    left_files.sort();
    right_files.sort();
    if left == right && left_files == right_files {
        Ok(())
    } else {
        Err(Difference {
            reference: left,
            candidate: right,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::diagnostics::types::Diagnostic;
    #[test]
    fn duplicates_are_not_discarded() {
        let finding = Diagnostic {
            rule: "E722".into(),
            file: "a.py".into(),
            start: 0,
            end: 6,
        };
        let one = RunReport {
            files: vec!["a.py".into()],
            diagnostics: vec![finding.clone()],
        };
        let two = RunReport {
            files: one.files.clone(),
            diagnostics: vec![finding.clone(), finding],
        };
        assert!(compare(&one, &two).is_err());
    }
}
