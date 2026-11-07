//! The `file_package` relation of the language: the module each file of the repository belongs to, the one whose directory is the nearest
//! above it.

use crate::domain::links::service::modules_in;
use crate::domain::links::types::Linked;
use crate::domain::modules::service::placed_in;
use crate::domain::modules::types::{FilePlacement, Wired};
use ryo::languages;

/// The reader the `project` contract hands the `file_package` relation to, which reads its rows from the repository's manifests.
pub(crate) struct FilePackageItem;

// A manifest the tool refused declares nothing, so the files it would have held are missing.
ryo::reads! {
    FilePackageItem => languages::FilePackage, manifests(Linked) |input, symbols|
    model wired: Wired<FilePlacement> = (Some(placed_in(&input.request.files, modules_in(input)))) {
        each (&wired.refused) as refused {
            partial refused.0 => languages::Missing::Any;
        }
        each (&wired.rows) as placed {
            row { path: value(placed.path), package: value(placed.module) }
        }
    }
}
