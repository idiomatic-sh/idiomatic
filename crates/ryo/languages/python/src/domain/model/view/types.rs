//! The types of the model's `view` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::model::definitions::types::DefinitionModel;
use crate::domain::model::flow::types::FlowModel;
use crate::domain::model::lexical::types::LexicalModel;
use crate::domain::model::lexical::types::TypedTree;
use crate::domain::model::names::types::{FactWords, NameTable};
use crate::domain::model::resolution::types::ResolutionModel;
use crate::domain::model::types::BindingSchedule;
use crate::domain::standard_library::types::TargetVersion;
use ryo::languages::{FileId, NodeTable};
use std::cell::RefCell;

/// The per-file view as the semantic fillers read it.
pub(crate) trait ModelView {
    /// The file's model with at least `part` built: built by the first fill that asks, inside the walk's visit frame.
    fn semantics(&mut self, part: ModelPart) -> ModelRead<'_>;
}

/// Which part of the model a filler reads: the flow part needs the lexical part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ModelPart {
    /// No part: the filler reads the table and the file's number alone, and the model is left as it is.
    Nothing,
    /// Scopes, names, declarations and definitions.
    Lexical,
    /// The lexical part, and the one definition the static rule gives every name that is read.
    Resolution,
    /// The lexical part, and the schedule interpreted: events, reads, uses and shadows.
    Flow,
    /// The flow part, and its reads listed by the name each reads: what a fill of the reads of one name looks its name up in.
    ReadsByName,
    /// The flow part, and its shadows listed by the binding of each: what a fill of the shadows of one binding looks it up in.
    ShadowsByBinding,
}

/// What a filler reads once the model is built.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ModelRead<'v> {
    /// The model.
    pub model: &'v SemanticModel,
    /// The file's node table.
    pub table: &'v NodeTable,
    /// The parser's nodes, as the model read them.
    pub tree: TypedTree<'v, 'v>,
    /// The file's number in the run.
    pub file: FileId,
}

/// The model of one file: per-worker buffers, cleared and refilled per file, never reallocated.
#[derive(Clone, Debug, Default)]
pub(crate) struct SemanticModel {
    /// The language target the file is read for: the walk sets it from its settings when it makes the view.
    pub target: TargetVersion,
    /// The names of the file.
    pub names: NameTable,
    /// What each node is to the name rules, and the scope it is evaluated in.
    pub lexical: LexicalModel,
    /// The definitions, and the static resolution of the names that are read.
    pub definitions: DefinitionModel,
    /// The one definition each name that is read resolves to.
    pub resolution: ResolutionModel,
    /// The names in evaluation order.
    pub schedule: BindingSchedule,
    /// The schedule interpreted.
    pub flow: FlowModel,
    /// The words the fillers have written for the file in hand, kept from one filler to the next.
    pub words: RefCell<FactWords>,
}

/// Whether a part of the model is built for the file in hand.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum PartState {
    /// Not yet.
    #[default]
    Pending,
    /// It is.
    Ready,
}

/// Which parts of the model this file has built so far; a view made for a file starts with none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Built {
    /// The lexical part.
    pub lexical: PartState,
    /// The static resolution.
    pub resolution: PartState,
    /// The flow part.
    pub flow: PartState,
    /// The reads listed by name.
    pub reads_by_name: PartState,
    /// The shadows listed by binding.
    pub shadows_by_binding: PartState,
}
