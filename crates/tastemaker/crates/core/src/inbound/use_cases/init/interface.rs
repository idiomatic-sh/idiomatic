//! The workspace initialization contract.
use super::errors::InitError;
use super::types::{InitReport, InitRequest};
/// Initializes an existing project without any external package.
pub trait InitUseCase: Send + Sync {
    /// Writes a local workspace and its asset package, preserving existing files.
    ///
    /// # Errors
    /// Returns `Exists` if either configuration already exists, or a repository error on IO failure.
    fn init(&self, request: InitRequest) -> Result<InitReport, InitError>;
}
