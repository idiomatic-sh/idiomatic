//! Load the explicit profile; project configuration never changes its defaults.
use ryo_benchmark::domain::{diagnostics::errors::Error, suite::types::Tool};
use std::path::{Path, PathBuf};

pub(crate) fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks/linters/ruff")
}

pub(crate) fn load() -> Result<Tool, Error> {
    ryo_benchmark::outbound::suite::adapters::local::schemas::tool(&std::fs::read_to_string(
        directory().join("benchmark.toml"),
    )?)
}
