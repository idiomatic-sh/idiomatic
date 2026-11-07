//! The words of the `constraints` concept: how a build constraint and a comment are written.

/// What opens a `//go:build` line.
pub(crate) const GO_BUILD: &[u8] = b"//go:build";

/// What opens a `// +build` line once its `//` and the space after it are dropped.
pub(crate) const PLUS_BUILD: &[u8] = b"+build";

/// The name of a `//go:build` line as the `directive` relation gives it: what follows the slashes up to its argument.
pub(crate) const GO_BUILD_KIND: &[u8] = b"go:build";

/// The name of a `// +build` line as the `directive` relation gives it.
pub(crate) const PLUS_BUILD_KIND: &[u8] = b"+build";

/// What opens a line comment.
pub(crate) const SLASH_SLASH: &[u8] = b"//";

/// What opens a block comment.
pub(crate) const SLASH_STAR: &[u8] = b"/*";

/// What closes a block comment.
pub(crate) const STAR_SLASH: &[u8] = b"*/";

/// What ends a line.
pub(crate) const NEWLINE: u8 = b'\n';
