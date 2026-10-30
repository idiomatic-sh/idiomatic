//! The types of the `keeping` concept: plain data, public fields and derives, with no constructor
//! and no inherent impl.

use ryo_language::domain::relations::types::{FileId, WorkerNumber};

use crate::domain::rules::types::RuleSpec;

/// How the numbers a file's words carry change when they are used in another run: the file has another
/// number, and the text its names made is somewhere else in another worker's arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Remap {
    /// The number the file had.
    pub from_file: FileId,
    /// The number it has.
    pub to_file: FileId,
    /// The worker whose arena held the text.
    pub from_worker: WorkerNumber,
    /// The worker whose arena holds it now.
    pub to_worker: WorkerNumber,
    /// Where the file's text began in the old arena.
    pub from_base: ArenaStart,
    /// Where it begins in the new one.
    pub to_base: ArenaStart,
}

/// Where the text of a file begins in a worker's arena, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ArenaStart(pub u32);

/// What is read of the rules to say whether what the store kept of them can be used: a rule's digest is the digest of these
/// bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Configuration<'a> {
    /// The rules, in the order they were given: the id, the query, the parameters as they are written, and the globs.
    pub rules: &'a [RuleSpec],
    /// The files the host said were placed, one line each: they are rows a rule reads, so another set is another configuration.
    pub hosted: &'a [HostedText],
}

/// A file the host said was placed, as one line of text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct HostedText(pub Box<str>);

/// A piece of text to digest, with its length.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DigestPart<'a>(pub &'a str);

/// Bytes to digest: what a rule's findings depend on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct DigestInput(pub Vec<u8>);
