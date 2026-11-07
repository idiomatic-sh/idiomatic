//! The types of the `language` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// Which of the language's files a path names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GoFile {
    /// A go source, read with the go grammar.
    Source,
    /// A module's `go.mod`, read for its directives as `golang.org/x/mod/modfile` reads them.
    Module,
    /// Neither: the language declines it.
    Other,
}
