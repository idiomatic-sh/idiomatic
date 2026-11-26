//! The `syntax_child` relation of the language: every edge of the tree, ordered under its parent.

use crate::domain::model::types::ReadIndexes;
use crate::domain::syntax::service::{is_root, ordinal, parent};
use crate::domain::syntax::types::Rootness;
use ryo::languages;

/// The reader the `fills_rows!` below binds the `syntax_child` relation's rows to.
pub(crate) struct SyntaxChildItem;
