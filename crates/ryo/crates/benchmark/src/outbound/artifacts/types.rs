//! The types of the `artifacts` port: plain data, public fields and derives (`Clone`, `Debug`, `PartialEq`), with no constructor and no inherent impl, and no handle, file, lock or `dyn`: a handle is a trait in interface.rs.
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use std::path::PathBuf;

/// A sealed executable: the copy an operation runs and the digest of its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sealed {
    /// Where the sealed copy is.
    pub path: PathBuf,
    /// The SHA-256 of its bytes.
    pub sha256: Sha256Text,
}
