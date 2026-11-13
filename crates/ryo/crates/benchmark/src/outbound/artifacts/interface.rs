//! What the `artifacts` port asks of the outside, in the core's own vocabulary: plain data in and out.
//! It never takes an adapter's configuration (the build tool, the search path): the port's factory takes that.
use super::types::Sealed;
use crate::domain::operations::types::BenchDirectory;
use crate::inbound::use_cases::operate::types::OperationVerdict;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::workloads::types::tool_name::value::ToolName;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// What every `artifacts` adapter implements: where builds go, the reference tool, sealed copies and their runs.
pub trait ArtifactsPort {
    /// The build target directory every operation writes under.
    ///
    /// # Errors
    /// Refuses a workspace whose metadata cannot be read.
    fn target(&self) -> Result<PathBuf, Error>;
    /// The reference tool's executable: the first one of that name on the search path, canonicalized.
    ///
    /// # Errors
    /// Refuses a name no directory of the search path holds, naming it.
    fn reference(&self, tool: &ToolName) -> Result<PathBuf, Error>;
    /// Build the bench's worker and seal exactly one bench executable at `sealed`, in a fresh directory: whatever
    /// the directory of `sealed` held is removed first, so every operation starts from nothing.
    ///
    /// # Errors
    /// Refuses a failed build, no or several bench executables, and a platform where a nested build blocks.
    fn seal_worker(&self, bench: &BenchDirectory, sealed: &Path) -> Result<Sealed, Error>;
    /// Seal the running executable at `sealed`.
    ///
    /// # Errors
    /// Refuses an unreadable executable or an unwritable destination.
    fn seal_current(&self, sealed: &Path) -> Result<Sealed, Error>;
    /// Run a sealed executable, copying its standard output to `output` as it arrives; its errors stream through.
    ///
    /// # Errors
    /// Refuses an executable that cannot start, and a failed copy of its output.
    fn run_sealed(
        &self,
        executable: &Path,
        arguments: &[OsString],
        output: &mut dyn std::io::Write,
    ) -> Result<OperationVerdict, Error>;
}
