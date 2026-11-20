//! The types of the `file_view` concept: the Python language as the engine's walk and its fillers name it, the view of one walked
//! file the fillers read, and the buffers a worker keeps from file to file.

use crate::domain::model::view::service::built;
use crate::domain::model::view::types::{Built, ModelPart, ModelRead, ModelView, SemanticModel};
use crate::domain::node::types::SyntaxFile;
use crate::domain::owner_facts::types::Owners;
use crate::domain::standard_library::types::TargetVersion;
use ryo::languages::ByteSlice;

/// The Python language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item
/// names the language with none (`PythonWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PythonWalk<P = ()> {
    /// The ports the walk calls.
    pub ports: P,
}

/// One walked file as the fillers read it: its syntax in place, the language version it is read under, and its semantic model,
/// built by the first fill that needs it.
#[derive(Debug)]
pub(crate) struct PythonFile<'f> {
    /// The node table, the typed node of each syntax node and the text.
    pub syntax: SyntaxFile<'f>,
    /// The file's bytes.
    pub bytes: ByteSlice<'f>,
    /// The language version the file is read under.
    pub target: TargetVersion,
    /// The semantic model, lent from the worker's buffers.
    pub model: &'f mut SemanticModel,
    /// The parts of the model this file has built.
    pub built: Built,
    /// The scope each node is evaluated in, worked out by the first filler that names an owner.
    pub owners: &'f mut Owners,
}

impl ModelView for PythonFile<'_> {
    fn semantics(&mut self, part: ModelPart) -> ModelRead<'_> {
        built(
            self.model,
            &mut self.built,
            part,
            self.syntax.table,
            self.syntax.nodes,
        );
        ModelRead {
            model: self.model,
            table: self.syntax.table,
            source: self.bytes,
            file: self.syntax.file,
        }
    }
}

/// The buffers a worker keeps from file to file: refilled for each file, never reallocated.
#[derive(Debug, Default)]
pub(crate) struct PythonScratch {
    /// The semantic model's buffers.
    pub model: SemanticModel,
    /// The owners' buffer.
    pub owners: Owners,
}
