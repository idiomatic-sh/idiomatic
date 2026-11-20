//! The types of the model's per-file view: the seam between the walk's file and the semantic fillers. Track P1's placeholder of the
//! file track P2 writes: the model holds no column and builds nothing here.

use ryo::languages::{ByteSlice, FileId, NodeTable};

/// The per-file view as the semantic fillers read it.
pub(crate) trait ModelView {
    /// The file's model with at least `part` built: built by the first fill that asks, inside the walk's visit frame.
    fn semantics(&mut self, part: ModelPart) -> ModelRead<'_>;
}

/// Which part of the model a filler reads: the flow part needs the lexical part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModelPart {
    /// Scopes, names and definitions.
    Lexical,
    /// Binding events and reads in evaluation order.
    Flow,
}

/// What a filler reads once the model is built.
#[expect(
    dead_code,
    reason = "track P1's placeholder: the fillers that read the model are the ones track P2 writes"
)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ModelRead<'v> {
    /// The file's model.
    pub model: &'v SemanticModel,
    /// The file's node table.
    pub table: &'v NodeTable,
    /// The file's bytes.
    pub source: ByteSlice<'v>,
    /// The number the run gave the file.
    pub file: FileId,
}

/// The model of one file: per-worker buffers, cleared and refilled per file.
#[derive(Clone, Debug, Default)]
pub(crate) struct SemanticModel {
    /// The parts asked for since the worker began: the placeholder builds none of them.
    pub asked: Vec<ModelPart>,
}

/// Which parts of the model this file has built so far.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Built {
    /// The last part a filler asked for; none before the first fill of the file.
    pub asked: Option<ModelPart>,
}
