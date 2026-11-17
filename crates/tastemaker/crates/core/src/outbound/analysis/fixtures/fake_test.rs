//! The `analysis` port's fake runs the port's contract.

use super::super::contract::check;
use super::AnalysisFake;
use tastemaker_values::domain::files::types::MachinePath;

#[test]
fn succeeds() {
    check(&AnalysisFake::default(), &MachinePath("/fake".into()));
}
