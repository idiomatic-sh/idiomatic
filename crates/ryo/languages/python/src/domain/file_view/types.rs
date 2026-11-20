//! The types of the `file_view` concept: the Python language as the engine's walk and its fillers name it, the view of one walked
//! file the fillers read, and the buffers a worker keeps from file to file.

use crate::domain::constant_keys::types::{KeyClass, KeyWord};
use crate::domain::expression_types::types::Resolution;
use crate::domain::format_specification::types::OwnedField;
use crate::domain::indentation::types::IndentationRegion;
use crate::domain::layout_facts::types::FileLines;
use crate::domain::literal_facts::types::TextBuffer;
use crate::domain::model::view::service::built;
use crate::domain::model::view::types::{Built, ModelPart, ModelRead, ModelView, SemanticModel};
use crate::domain::node::types::{FixedWords, SyntaxFile};
use crate::domain::standard_library::types::TargetVersion;
use crate::domain::token_links::types::Opener;
use ryo::languages::{Position, RetainedKeyIndex, ShapeWord};

/// The Python language as the engine walks it, over the ports `P` its walk calls. Its fillers call no port, so a `fills!` item
/// names the language with none (`PythonWalk`), and the same fillers serve the walk an edge composes over its ports.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PythonWalk<P = ()> {
    /// The ports the walk calls.
    pub(crate) ports: P,
}

/// One walked file as the fillers read it: its syntax in place, the language version it is read under, and its semantic model,
/// built by the first fill that needs it.
#[derive(Debug)]
pub(crate) struct PythonFile<'f> {
    /// The node table, the typed node of each syntax node and the text.
    pub(crate) syntax: SyntaxFile<'f>,
    /// The language version the file is read under.
    pub(crate) target: TargetVersion,
    /// The semantic model, lent from the worker's buffers.
    pub(crate) model: &'f mut SemanticModel,
    /// The parts of the model this file has built.
    pub(crate) built: Built,
    /// The buffers the fillers write through, lent from the worker's.
    pub(crate) buffers: &'f mut FillBuffers,
}

impl ModelView for PythonFile<'_> {
    fn semantics(&mut self, part: ModelPart) -> ModelRead<'_> {
        let tree = self.syntax.tree;
        built(self.model, &mut self.built, part, tree);
        ModelRead {
            model: self.model,
            table: self.syntax.table,
            tree,
            file: self.syntax.file,
        }
    }
}

/// The buffers a worker keeps from file to file: refilled for each file, never reallocated.
#[derive(Debug, Default)]
pub(crate) struct PythonScratch {
    /// The semantic model's buffers.
    pub(crate) model: SemanticModel,
    /// The fillers' buffers.
    pub(crate) buffers: FillBuffers,
}

/// The buffers the fillers of a file write through: kept by the worker from file to file, so a fill allocates none of its own.
#[derive(Clone, Debug, Default)]
pub(crate) struct FillBuffers {
    /// A text a filler builds for one row.
    pub(crate) text: TextBuffer,
    /// The fixed words this file has interned.
    pub(crate) fixed: FixedWords,
    /// The open brackets of a scan of the tokens.
    pub(crate) brackets: Vec<Opener>,
    /// The indentation regions of the file.
    pub(crate) regions: Vec<IndentationRegion>,
    /// Where the lines of the file start.
    pub(crate) lines: FileLines,
    /// The basic type of each node of the file.
    pub(crate) types: Vec<Resolution>,
    /// The constants of the file by their value.
    pub(crate) keys: RetainedKeyIndex<KeyClass, KeyWord>,
    /// The expressions of the file by their structure.
    pub(crate) shapes: RetainedKeyIndex<Position, ShapeWord>,
    /// The interpolations of the file with the f-string that holds each.
    pub(crate) owned: Vec<OwnedField>,
}
