//! What the conformance holds the adapter to, which only tests and the `fixtures` feature use: the library's own walk of a tree and
//! its own reading of the items of a file.

pub(crate) mod oracle;
pub(crate) mod walk;

use super::adapter::{concrete, parser};
use crate::domain::grammar::types::{Concrete, GoText};

/// The library's tree of `text` as plain data in the words of Go: what the services of the domain are run on; none when the grammar
/// and the library disagree on their versions or the library gives no tree.
pub(crate) fn concreted(text: GoText<'_>) -> Option<Concrete<'_>> {
    let tree = parser().ok()?.parse(text.0, None)?;
    Some(concrete(&tree, text, &mut Vec::new()))
}
