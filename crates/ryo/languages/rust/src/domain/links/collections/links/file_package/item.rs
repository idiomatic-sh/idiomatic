//! The `file_package` relation of the language: the package each file of the repository belongs to, the innermost one whose
//! directory holds it.

use crate::domain::links::service::{file_id, packaged_in};
use crate::domain::links::types::{Linked, Packaged};
use crate::domain::packages::service::refusal_cause;
use ryo::languages;

/// The reader the `ryo::reads!` below binds the `file_package` relation's rows to.
pub(crate) struct FilePackageItem;

// A manifest Cargo refused declares nothing, so the files it would have held are missing.
ryo::reads! {
    FilePackageItem => languages::FilePackage, manifests(Linked) |input, symbols|
    model packaged: Packaged = (packaged_in(input)) {
        each (&packaged.refused) as refused {
            partial refusal_cause(refused).to_string() => languages::Missing::Any;
        }
        each (&packaged.files) as placed {
            given package = packaged.listed.get(&placed.package).copied()?;
            row {
                path: value(file_id(placed.place)),
                package: value(package),
            }
        }
    }
}
