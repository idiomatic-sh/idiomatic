//! The names of the `declarations` concept: which nodes declare their own name, and the note of a name declared more than once.

use crate::domain::nodes::types::Node;
use crate::domain::refinements::types::Declared;
use crate::domain::syntax::types::Kind;
use ryo::languages;

/// The nodes that declare their own `NAME`, and what each declares; a kind not here declares nothing, or has its name read apart.
pub(crate) const NAMED: [(Kind, Declared); 14] = [
    (Kind::Node(Node::Struct), Declared::Struct),
    (Kind::Node(Node::Enum), Declared::Enum),
    (Kind::Node(Node::Variant), Declared::Variant),
    (Kind::Node(Node::RecordField), Declared::Field),
    (Kind::Node(Node::Trait), Declared::Trait),
    (Kind::Node(Node::Const), Declared::Const),
    (Kind::Node(Node::Static), Declared::Static),
    (Kind::Node(Node::Module), Declared::Module),
    (Kind::Node(Node::TypeAlias), Declared::TypeAlias),
    (Kind::Node(Node::Union), Declared::Union),
    (Kind::Node(Node::MacroRules), Declared::MacroRules),
    (Kind::Node(Node::MacroDef), Declared::Macro),
    (Kind::Node(Node::TypeParam), Declared::TypeParameter),
    (Kind::Node(Node::ConstParam), Declared::ConstParameter),
];

/// The note of a name that more than one `struct` or `enum` of the file declares.
pub(crate) const MANY: languages::NameNote = languages::NameNote(u32::MAX);
