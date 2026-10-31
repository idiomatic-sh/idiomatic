//! Measured wall time covers the complete child process, including startup.
use crate::domain::diagnostics::types::RunReport;
use std::time::Duration;

/// An individual process observation. No CPU or RSS estimate is fabricated.
#[derive(Clone, Debug)]
pub struct Measurement {
    pub wall: Duration,
    pub report: RunReport,
}

use std::ffi::OsString;
use std::path::PathBuf;
#[derive(Clone, Debug)]
pub struct Invocation {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
    pub directory: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub wall: Duration,
}

#[derive(Clone, Debug)]
pub struct Pair {
    pub reference: Measurement,
    pub candidate: Measurement,
}
