//! The test support of the `kind_index` concept: the grouping of a table by kind the tests of other crates ask for.

use super::service;
use crate::domain::elements::types::Elements;

/// Groups the positions of a table by kind.
pub fn index_kinds(elements: &mut Elements) {
    service::index_kinds(elements);
}
