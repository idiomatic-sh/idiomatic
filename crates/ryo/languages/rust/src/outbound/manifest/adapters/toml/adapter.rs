//! The manifest port, implemented with `toml`: a `Cargo.toml` and a `.cargo/config.toml` are read into the port's own types. The
//! library's names stop here.

use crate::domain::paths::types::FileText;
use crate::domain::targets::types::Config;
use crate::domain::targets::types::Manifest;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;

/// The `toml` manifest adapter.
#[derive(Debug)]
pub(crate) struct ManifestTomlAdapter;

impl ManifestPort for ManifestTomlAdapter {
    fn read(&self, text: &FileText) -> Result<Manifest, ManifestError> {
        Manifest::try_from(&*text.0)
    }

    fn configuration(&self, text: &FileText) -> Result<Config, ManifestError> {
        Config::try_from(&*text.0)
    }
}
