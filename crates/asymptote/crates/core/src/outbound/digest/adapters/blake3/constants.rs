//! The words the item fingerprint reads an item's declaration by: the item keywords every lock's keys rest on, and the declarations of `function`, `func` and their kin beside them.

/// The words that open an item's declaration as the fingerprint reads them: each may be followed by anything that
/// does not continue the word.
pub(super) const ITEMS: [&str; 11] = [
    "fn",
    "struct",
    "enum",
    "union",
    "trait",
    "impl",
    "mod",
    "type",
    "macro_rules!",
    "def",
    "class",
];
/// The words that open an item's declaration followed by whitespace: a call such as `func(x)` declares nothing.
pub(super) const SPACED_ITEMS: [&str; 5] =
    ["function", "function*", "func", "interface", "namespace"];
/// The words that may come before an item's keyword without naming it: the common ones, then the modifiers of a
/// class's members and of an exported item.
pub(super) const QUALIFIERS: [&str; 18] = [
    "pub",
    "async",
    "unsafe",
    "const",
    "extern",
    "default",
    "\"C\"",
    "export",
    "static",
    "abstract",
    "declare",
    "override",
    "private",
    "protected",
    "public",
    "readonly",
    "get",
    "set",
];
/// The words that bind a name, which declares a function when it is bound to one.
pub(super) const BINDINGS: [&str; 3] = ["const", "let", "var"];
/// The word a bound function's value may open with, besides a parenthesized or bare parameter before `=>`.
pub(super) const FUNCTION: &str = "function";
/// The word that makes a bound function asynchronous.
pub(super) const ASYNC: &str = "async";
/// The arrow of an arrow function.
pub(super) const ARROW: &str = "=>";
/// The words that are followed by a parenthesis and a body without declaring a method.
pub(super) const CONTROL: [&str; 14] = [
    "if", "for", "while", "switch", "catch", "with", "return", "else", "do", "try", "new", "await",
    "match", "loop",
];
/// The keyword a method, `name(...) {`, is recorded under.
pub(super) const METHOD: &str = "method";
/// What opens a body.
pub(super) const BODY: &str = "{";
/// The keyword whose name is everything up to its body or `where` clause.
pub(super) const IMPL: &str = "impl";
/// The word that opens a `where` clause.
pub(super) const WHERE: &str = "where";
/// A `where` clause after an impl's name.
pub(super) const SPACED_WHERE: &str = " where";
/// A restricted visibility, `pub(crate)`, which qualifies too.
pub(super) const PUB_OPEN: &str = "pub(";
