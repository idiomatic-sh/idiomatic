//! Files belong to the nearest enclosing package manifest.

use crate::domain::links::service::packages_in;
use crate::domain::links::types::Linked;
use crate::domain::packages::service::ownership;
use crate::domain::packages::service::projected;
use crate::domain::packages::types::Projection;
use ryo::languages;

/// Answers the `file-package` rows of the repository's package manifests and files.
pub(crate) struct FilePackageItem;

ryo::reads! {
    FilePackageItem => languages::FilePackage, manifests(Linked) |input, symbols|
    model projection: Projection<languages::FilePackageRow> = (Some(projected(packages_in(input), ownership(&input.request.files, packages_in(input))))) {
        each (&projection.refused) as refused {
            partial refused.reason.0 => languages::Missing::Any;
        }
        each (&projection.rows) as ownership {
            row { path: value(ownership.path), package: value(ownership.package) }
        }
    }
}
