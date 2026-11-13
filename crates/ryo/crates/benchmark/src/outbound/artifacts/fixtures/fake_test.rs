//! The `artifacts` port's fake runs the port's contract.

use super::super::contract::check;
use super::ArtifactsFake;
use crate::inbound::use_cases::operate::types::OperationVerdict;

#[test]
fn succeeds() {
    check(&ArtifactsFake {
        target: std::path::PathBuf::from("target"),
        tools: Vec::new(),
        stdout: Vec::new(),
        verdict: OperationVerdict::Passed,
        history: std::sync::Mutex::default(),
    });
}
