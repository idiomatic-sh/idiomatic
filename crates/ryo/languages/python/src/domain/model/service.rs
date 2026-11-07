//! The `model` service: free functions named by verb over the file model, with no IO and no ports.

use ryo::languages::{Position, Symbol};

use super::types::{BindingRedirect, Declarations};

/// Where the scope `scope` says the name `name` is bound instead of in itself, when a `global` or a `nonlocal` of it says so.
#[must_use]
pub(crate) fn redirect_of(
    model: &Declarations,
    scope: Position,
    name: Symbol,
) -> Option<BindingRedirect> {
    model
        .declared
        .borrow()
        .iter()
        .find(|each| each.scope == scope && each.name == name)
        .map(|each| each.redirect)
}
