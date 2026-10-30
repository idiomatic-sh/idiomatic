//! How the language shows each kind of its trees that is one the standard lacks.

use crate::domain::extras::types::Extra;
use ryo::languages::{Sample, Why};

/// How the language shows an extra kind of its trees: the source whose tree holds it, or the reason no relation models it.
#[must_use]
pub(super) const fn sample(kind: Extra) -> Sample {
    match kind {
        Extra::Other => Sample::Unread(Why(
            "a marker, not a kind of element: it stands for the parser's TOMBSTONE, EOF and __LAST markers, which no tree it builds holds",
        )),
    }
}
