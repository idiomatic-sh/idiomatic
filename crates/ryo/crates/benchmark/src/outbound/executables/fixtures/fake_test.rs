//! The `executables` port's fake runs the port's contract.

use super::super::contract::check;
use super::ExecutablesFake;
use crate::outbound::executables::types::RunVerdict;

#[test]
fn succeeds() {
    check(&ExecutablesFake {
        target: std::path::PathBuf::from("target"),
        tools: Vec::new(),
        stdout: Vec::new(),
        verdict: RunVerdict::Passed,
        history: std::sync::Mutex::default(),
    });
}
