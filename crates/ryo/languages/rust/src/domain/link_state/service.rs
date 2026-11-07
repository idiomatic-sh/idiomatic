//! The `link_state` service: what the language hands from its resolving of the project to the readers of the manifests' relations: what
//! the manifests declared, read in process on every run. Nothing of it is kept for another run.

use crate::domain::packages::service::numbered;
use crate::domain::packages::types::{Located, Packages};
use ryo::languages::FilePath;

/// What the manifests declared, with the number of the file of each manifest they name.
#[must_use]
pub(crate) fn located_of(files: &[FilePath], packages: &Packages) -> Located {
    let listed = numbered(files, packages);
    Located {
        packages: packages.clone(),
        listed,
    }
}
