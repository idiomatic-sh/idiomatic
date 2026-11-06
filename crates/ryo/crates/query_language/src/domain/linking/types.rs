//! Plain linker indexes over parsed query bundle values.

use std::collections::BTreeMap;

use ryo_values::domain::program::types::{ArgumentValue, ImportDeclaration};

/// The local alias an import is visible under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ImportAlias(pub Box<str>);

impl std::borrow::Borrow<str> for ImportAlias {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Imported aliases visible from one source document.
#[derive(Clone, Debug, Default)]
pub(super) struct ImportIndex(pub BTreeMap<ImportAlias, ImportDeclaration>);

/// The name a fragment input or parameter is bound under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FragmentBindingKey(pub Box<str>);

impl std::borrow::Borrow<str> for FragmentBindingKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Substitutions for a fragment's relation and scalar inputs.
#[derive(Clone, Debug, Default)]
pub(super) struct Bindings(pub BTreeMap<FragmentBindingKey, ArgumentValue>);

/// Whether a call's parameters must all be declared by the fragment that makes the call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ParameterCheck {
    /// An undeclared parameter is refused.
    Strict,
    /// An undeclared parameter is left for the caller's own scope to answer.
    Lenient,
}

/// Whether an argument has the type its parameter declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScalarMatch {
    /// The argument is of the declared type.
    Matches,
    /// The argument is of another type, or the declared type is unknown.
    Differs,
}

/// The two binding sets of one call: the relation inputs and the scalar parameters.
#[derive(Clone, Debug)]
pub(super) struct BoundArguments {
    /// The relation inputs.
    pub inputs: Bindings,
    /// The scalar parameters.
    pub parameters: Bindings,
}
