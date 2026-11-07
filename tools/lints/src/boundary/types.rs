#[derive(Clone, Copy)]
pub(crate) enum BoundarySite {
    Declaration,
    Implementation,
    Receiver,
}

/// The representation types an interface boundary never resolves to, however it is
/// written: a flag, a character, text, a number, a std collection or an absence says
/// nothing about what it stands for, so the root's `types.rs` declares a nominal type that
/// does (a newtype with a checked constructor, a two-state enum, a collection that owns its
/// invariant, an enum variant that names the absence). The policy is the lint's own; no
/// configuration file extends or weakens it.
pub(crate) const REPRESENTATIONS: [&str; 27] = [
    "bool",
    "char",
    "str",
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
    "f32",
    "f64",
    "std::string::String",
    "std::vec::Vec",
    "std::collections::VecDeque",
    "std::collections::LinkedList",
    "std::collections::HashMap",
    "std::collections::HashSet",
    "std::collections::BTreeMap",
    "std::collections::BTreeSet",
    "std::collections::BinaryHeap",
    "core::option::Option",
];
