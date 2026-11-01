//! A source file's language-defined importable module path, established by project context.
use super::super::interface::RelationsItem;
use crate::domain::{relations::types::FileId, terms::types::Symbol};
/// Optional project-produced module identity.
#[derive(Clone, Copy, Debug)]
pub struct ModuleIdentity;
impl super::super::interface::sealed::Sealed for ModuleIdentity {}
/// Declare the module identity row and its real source-file key.
#[macro_export]
#[doc(hidden)]
macro_rules! module_identity {
    ($callback:path) => {
        $callback! {
            item ModuleIdentity,
            row { /// One known importable source module.
                ModuleIdentityRow },
            column { /// Module identity columns.
                ModuleIdentityColumn },
            expected { /// Authored module identity row.
                ModuleIdentityExpectedRow },
            class Manifest,
            columns {
                /// Actual requested source file.
                file File: FileId => File;
                /// Language-defined dotted module path.
                path Path: Symbol => Symbol;
            }
        }
    };
}
crate::module_identity!(crate::relation_row);
