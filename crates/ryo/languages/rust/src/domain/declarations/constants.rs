//! The names of the `declarations` concept: which nodes declare their own name, and the note of a name declared more than once.

use crate::domain::nodes::types::RustNode;
use crate::domain::refinements::types::Declared;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The nodes that declare their own `NAME`, and what each declares; a kind not here declares nothing, or has its name read apart.
pub(crate) const NAMED: [(Kind, Declared); 14] = [
    (Kind::Node(RustNode::Struct), Declared::Struct),
    (Kind::Node(RustNode::Enum), Declared::Enum),
    (Kind::Node(RustNode::Variant), Declared::Variant),
    (Kind::Node(RustNode::RecordField), Declared::Field),
    (Kind::Node(RustNode::Trait), Declared::Trait),
    (Kind::Node(RustNode::Const), Declared::Const),
    (Kind::Node(RustNode::Static), Declared::Static),
    (Kind::Node(RustNode::Module), Declared::Module),
    (Kind::Node(RustNode::TypeAlias), Declared::TypeAlias),
    (Kind::Node(RustNode::Union), Declared::Union),
    (Kind::Node(RustNode::MacroRules), Declared::MacroRules),
    (Kind::Node(RustNode::MacroDef), Declared::Macro),
    (Kind::Node(RustNode::TypeParam), Declared::TypeParameter),
    (Kind::Node(RustNode::ConstParam), Declared::ConstParameter),
];

/// The note of a name that more than one `struct` or `enum` of the file declares.
pub(crate) const MANY: languages::NameNote = languages::NameNote(u32::MAX);
