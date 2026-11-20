//! One path an attribute writes, as one name.
use super::super::interface::RelationsItem;
use crate::domain::relations::types::{FileId, NodeId, Span};
use crate::domain::terms::types::Symbol;

/// `attribute-path`, one item of the `relations` collection.
#[derive(Clone, Copy, Debug)]
pub struct AttributePath;
impl super::super::interface::sealed::Sealed for AttributePath {}

/// The declaration of `attribute-path`.
#[macro_export]
#[doc(hidden)]
macro_rules! attribute_path {
    ($callback:path $(, $($extra:tt)*)?) => {
        $callback! {
            $( $($extra)* )?
            item AttributePath,
            row { /// One path an attribute writes.
                AttributePathRow },
            column { /// The columns of `attribute-path`.
                AttributePathColumn },
            expected { /// One expected row of `attribute-path`.
                AttributePathExpectedRow },
            class Code,
            scope File,
            notes Unneeded,
            form Filled(Many(Id)),
            size PerElement,
            keys [],
            columns {
                /// The element the path starts at: the path node of an attribute's meta, or the first token of a path written in an
                /// attribute's token tree.
                id Id: NodeId => Node;
                /// The path as written: its segments in order, each as the source spells it (`r#` kept), joined by `::`, a leading
                /// `::` kept, with comments and whitespace left out.
                name Name: Symbol => Symbol;
                /// The file it is in.
                file File: FileId => File;
                /// Where the path is, from its first segment or `::` to its last segment.
                span Span: Span => Span;
            }
        }
    };
}
crate::attribute_path!(crate::relation_row);
