//! Child-process execution contract.
use crate::domain::diagnostics::errors::Error;
use crate::domain::measurements::types::{Invocation, ProcessOutput};
pub trait ExecutionPort {
    fn run(&self, invocation: Invocation) -> Result<ProcessOutput, Error>;
}
