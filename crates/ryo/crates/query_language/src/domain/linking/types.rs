//! Plain linker indexes over parsed query bundle values.

use std::collections::BTreeMap;

use ryo_values::domain::program::types::{ArgumentValue, ImportDeclaration};

/// Imported aliases visible from one source document.
pub(super) type Imported = BTreeMap<Box<str>, ImportDeclaration>;
/// Substitutions for a fragment's relation and scalar inputs.
pub(super) type Bindings = BTreeMap<Box<str>, ArgumentValue>;
