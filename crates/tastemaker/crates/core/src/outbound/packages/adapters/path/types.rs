//! The types of the path packages adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::outbound::packages::types::HeldFiles;
use tastemaker_values::domain::packages::types::PackageRoot;

/// The packages the run made that the adapter holds in memory, by the directory each is read at.
#[derive(Debug, Default)]
pub(crate) struct HeldPackages(
    pub std::sync::Mutex<std::collections::BTreeMap<PackageRoot, HeldFiles>>,
);
