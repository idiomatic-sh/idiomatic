//! Builds the tools adapters, one function each. A factory builds nothing else, chooses nothing, and is called only by
//! an edge.
#![expect(
    clippy::pub_use,
    reason = "the edge opens this resource without exposing adapter implementation modules"
)]
pub use super::adapters::subprocess::connection::ToolsConnection;

/// The subprocess adapter: each tool a child process leading a process group of its own, so a timeout ends all of
/// it, noted in `connection` while it runs, so the edge that opened it can stop it with the command.
#[must_use]
pub fn subprocess(
    connection: ToolsConnection,
) -> impl crate::outbound::tools::interface::ToolsPort {
    super::adapters::subprocess::adapter::ToolsSubprocessAdapter { connection }
}
