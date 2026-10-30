//! The `file_package` relation of the language: the package each file of the repository belongs to, the one whose directory is the
//! nearest above it.

use crate::domain::links::service::packages_in;
use crate::domain::links::types::Linked;
use crate::domain::packages::service::placed_in;
use crate::domain::packages::types::{FilePlacement, Wired};
use ryo::languages;

/// Answers the `file-package` rows of the repository's Python manifests and files.
pub(crate) struct FilePackageItem;

// A manifest the tool refused declares nothing, so the files it would have held are missing.
ryo::reads! {
    FilePackageItem => languages::FilePackage, manifests(Linked) |input, symbols|
    model wired: Wired<FilePlacement> = (Some(placed_in(&input.request.files, packages_in(input)))) {
        each (&wired.refused) as refused {
            partial refused.0 => languages::Missing::Any;
        }
        each (&wired.rows) as placed {
            row { path: value(placed.path), package: value(placed.package) }
        }
    }
}
