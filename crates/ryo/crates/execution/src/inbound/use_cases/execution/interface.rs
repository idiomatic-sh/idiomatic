//! Required preparation boundary; executable ownership stays inside this crate.
use super::types::{PreparedProgram, PreparedRequest};
use crate::domain::operating::errors::OperatingError;
pub trait ExecutionUseCase {
    fn prepare(&self, request: PreparedRequest) -> Result<PreparedProgram, OperatingError>;
}
