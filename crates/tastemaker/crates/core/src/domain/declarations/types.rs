//! The types of the `declarations` concept: plain data, public fields and derives, with no constructor and no inherent
//! impl.

use std::collections::BTreeSet;

use crate::domain::contracts::types::{AttributeText, DeclarationName};

/// Whether a declaration's annotated type is part of what it declares: a value's is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Typing {
    /// It is: a value.
    Typed,
    /// It is not.
    Untyped,
}

/// What a declaration carries: its attributes, and the paths of its decorations.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Held {
    /// Its attributes.
    pub attributes: BTreeSet<AttributeText>,
    /// The paths of its decorations.
    pub paths: BTreeSet<DeclarationName>,
}
