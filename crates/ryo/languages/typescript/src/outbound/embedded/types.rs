//! The types of the `embedded` port: plain data, public fields and derives, with no constructor and no inherent impl.

/// The text of a file that carries typescript among other text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Markup<'a>(pub &'a str);

/// Where a block starts in the file, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Start(pub usize);

/// What a block is written in: plain script, TypeScript, or TypeScript with JSX, as the file marks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockLanguage {
    /// Plain JavaScript, with no `lang` or another one.
    Script,
    /// TypeScript: `lang="ts"`.
    TypeScript,
    /// TypeScript with JSX: `lang="tsx"`.
    Tsx,
}

/// The blocks of one file, in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Blocks<'a>(pub Vec<Block<'a>>);

/// One block of typescript in a file: its text, and where that text starts in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Block<'a> {
    /// The block's text, between its tags.
    pub text: Markup<'a>,
    /// Where the text starts in the file.
    pub start: Start,
    /// What the block is written in, as its tag marks it.
    pub dialect: BlockLanguage,
}
