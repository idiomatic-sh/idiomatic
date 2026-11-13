//! Explicit compiler-language modes computed by a language provider.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, Flag};
use crate::domain::terms::types::Symbol;
/// The optional `compiler-feature` relation.
#[derive(Clone, Copy, Debug)]
pub struct CompilerFeature;
impl super::super::interface::sealed::Sealed for CompilerFeature {}
/// Declare the shared feature row without interpreting language-specific feature names.
#[macro_export]
#[doc(hidden)]
macro_rules! compiler_feature {
    ($callback:path) => {
        $callback! {
            item CompilerFeature,
            row { /// One explicitly known feature value for a source file.
                CompilerFeatureRow },
            column { /// Compiler feature column names.
                CompilerFeatureColumn },
            expected { /// An independently expected compiler feature row.
                CompilerFeatureExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            columns {
                /// The source file whose compiler mode was computed.
                file File: FileId => File;
                /// A language-owned, namespaced compiler feature identifier.
                feature Feature: Symbol => Symbol;
                /// Explicit known enabled or disabled state; missing capability is unknown.
                enabled Enabled: Flag => Flag;
            }
        }
    };
}
crate::compiler_feature!(crate::relation_row);
