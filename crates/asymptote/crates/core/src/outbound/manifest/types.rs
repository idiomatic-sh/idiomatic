//! The types of the `manifest` port: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::manifest::types::{Hook, ProductRoot};

/// The member products of a workspace, sorted by path.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Products(pub Vec<ProductRoot>);

/// The hooks a workspace root maps to groups.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HookTable(pub Vec<Hook>);
