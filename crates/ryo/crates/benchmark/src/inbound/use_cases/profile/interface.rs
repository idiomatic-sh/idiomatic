//! A bench's command line, with the heap and CPU profiles recorded first when it asks for them.
use super::types::{ProfileReport, ProfileRequest};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
/// The profile run every linter bench reaches through its composition.
pub trait ProfileUseCase {
    /// Records the heap and CPU profiles when the arguments ask for `profile-ryo`; answers the command the bench runs next.
    ///
    /// # Errors
    /// A child fails, the two children's stdout differ, no heap dump is written, or a tool fails.
    fn profile(&self, request: ProfileRequest) -> Result<ProfileReport, ReportsError>;
}
