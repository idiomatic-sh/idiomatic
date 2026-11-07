// the vue and svelte adapters of the `embedded` port each scan their own markup for its script blocks: neither may name the other's constants (adapter-owns-no-adapter), and a markup tag is no concept of the language's domain
//! The text the svelte adapter reads with.

/// The ending of the paths of the files this adapter reads.
pub(super) const EXTENSION: &str = ".svelte";
/// What opens a script tag.
pub(super) const OPEN: &str = "<script";
/// What closes a script block.
pub(super) const CLOSE: &str = "</script>";
/// What opens a markup comment, inside which a script tag is not one.
pub(super) const COMMENT_OPEN: &str = "<!--";
/// What closes a markup comment.
pub(super) const COMMENT_CLOSE: &str = "-->";
/// What ends an opening tag.
pub(super) const TAG_END: &str = ">";
/// What a self-closing tag ends its attributes with: such a tag holds no block.
pub(super) const SELF_CLOSE: &str = "/";
/// The attribute that names the language of a script block.
pub(super) const LANG: &str = "lang";
/// The `lang` values of a typed block, which this language does not read as script.
pub(super) const TYPED: [&str; 2] = ["ts", "tsx"];
