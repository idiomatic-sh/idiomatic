//! What the `syntax` port asks of the outside: Rust source text in, the element table of its syntax tree out.
//!
//! # What an adapter makes
//!
//! The table is written through the builder the port is given, in the language's own vocabulary and in pre-order, the way a parser
//! walks its tree:
//!
//! | step                                  | call                                  |
//! | ------------------------------------- | ------------------------------------- |
//! | a node begins at byte `start`         | `enter_node(builder, kind_bits(kind), start)` |
//! | a token spans `start..end`            | `token(builder, kind_bits(kind), start, end)` |
//! | the node last entered ends at `end`   | `leave_node(builder, end)`            |
//! | the parser could not read the text    | `diagnose(builder, message, start, end)` |
//!
//! Every kind is a `Kind`: the language's own kinds, which no library's names reach. An adapter maps the kinds of its library by
//! a `match` with an arm for each of them and no catch-all, so a kind the library gains stops it compiling until it is given a kind
//! on purpose. The first element is the root, a `SourceFile` that spans the whole text even when the text is empty. White space and
//! comments are tokens like any other, so the tokens of a node spell its text exactly. A file with syntax errors still gives the
//! elements that could be read: the parser recovers from what it cannot, so reading never fails.
//!
//! # Roles
//!
//! The role an element plays under its parent (`Field`) is not the port's: it is read from the kinds of the tree afterwards, by the
//! domain's `fields` concept, so an adapter that makes the same kinds gets the same roles.
//!
//! # Macros
//!
//! A token tree has no structure of its own. The adapter reads the arguments of a macro call again, and adds the trees it finds after
//! the file's own tree, each under its call: `adopt(builder, call)`, then `mark_expansion(builder, ..)` for each element and the writes
//! of its steps, then `release(builder)`. A call is read when all of these hold:
//!
//! | condition | what is read, or why not |
//! | --- | --- |
//! | it is a macro call of a tree read: the file's own, or an argument read before | a `name!(..)` inside a token tree (a `macro_rules!` body, an argument not read) is no call |
//! | its token tree opens and closes with a matching delimiter pair | a call left open at the end of a file adds nothing |
//! | its name, the last segment of its path, is not one the adapter is built with as `OpaqueMacros` | the built-ins whose arguments are never code (`stringify!`, `cfg!`) |
//! | the whole token tree is of the call's grammar, with no error | each argument of a list of expressions (a named one, `width = 3`, as its value); `vec!`'s elements or element and count; `matches!`'s expression, pattern and guard |
//!
//! A call inside an argument is read the same way, under its own call. The file's own tree keeps the token tree as it was.
//!
//! # String literals
//!
//! A string literal's value (`"x\\y.rs"` is `x\y.rs`) is the library's to say, so the adapter writes a string token with the value its
//! library decodes from it (`decoded_token`), and the readers of the table read that. A string whose escape the library rejects is
//! written as an ordinary token, with no value.
//!
//! # The table-builder protocol
//!
//! | rule                                              | why                                            |
//! | ------------------------------------------------- | ---------------------------------------------- |
//! | enter and leave pair up, and tokens lie between    | the table is a tree: sizes and parents come from it |
//! | offsets only grow, and a child lies inside its parent | spans are the text a finding quotes         |
//! | nothing is written after `release`                | the expansions are the last elements           |
//!
//! The contract in `fixtures/contract.rs` holds every adapter to these with rows written by hand.

use super::types::SourceText;
use ryo::languages::TableBuilder;

/// What every `syntax` adapter implements.
pub(crate) trait SyntaxPort: Send + Sync {
    /// Reads Rust source into the table `builder` builds, a pre-order table of its nodes and tokens, each token's text interned as a
    /// place in `text`; the arguments of each macro call are read again under the call, but for the macros the adapter was built to hold
    /// opaque. Each byte of `text` that is not part of a character is read as a character the lexer reports, so the rest keeps its tree,
    /// at the same offsets.
    fn read(&self, text: SourceText<'_>, builder: &mut TableBuilder<'_>);
}
