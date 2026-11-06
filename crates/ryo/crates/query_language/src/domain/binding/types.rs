//! The types of the `binding` concept: plain data, public fields and derives.

use ryo_values::domain::query_syntax::types::ArgumentValue;
use std::collections::BTreeMap;

/// The name a fragment input or parameter is bound under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct FragmentBindingKey(pub Box<str>);

/// Substitutions for a fragment's relation and scalar inputs.
#[derive(Clone, Debug, Default)]
pub(crate) struct Bindings(pub BTreeMap<FragmentBindingKey, ArgumentValue>);

/// The two binding sets of one call: the relation inputs and the scalar parameters.
#[derive(Clone, Debug)]
pub(crate) struct BoundArguments {
    /// The relation inputs.
    pub inputs: Bindings,
    /// The scalar parameters.
    pub parameters: Bindings,
}

/// Whether an argument has the type its parameter declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScalarMatch {
    /// The argument is of the declared type.
    Matches,
    /// The argument is of another type, or the declared type is unknown.
    Differs,
}

impl std::borrow::Borrow<str> for FragmentBindingKey {
    fn borrow(&self) -> &str {
        &self.0
    }
}
