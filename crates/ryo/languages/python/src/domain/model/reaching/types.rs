//! The types of the model's `reaching` part: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::model::definitions::types::DefinitionModel;
use crate::domain::model::lexical::types::{LexicalModel, NamespaceKey, TypedTree};
use crate::domain::model::names::types::{KeyHasher, NameTable};
use crate::domain::model::types::{BindingSchedule, BindingStep};
use crate::domain::qualification::types::Version;
use ryo::languages::{Count, Position};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

/// What can reach a read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlowReadState {
    /// One definition.
    Unique,
    /// One of several definitions.
    Alternatives,
    /// A definition, or nothing.
    PossiblyUnbound,
    /// Nothing.
    Unbound,
    /// A builtin of the language, there since the version.
    Builtin(Version),
    /// A definition, or a builtin of the language.
    DeclarationOrBuiltin(Version),
    /// Unknown: a loop, a handler or a wildcard import leaves it open.
    Uncertain,
}

/// One list of mutations, by its place among the lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct MutationList(pub u32);

impl From<MutationList> for usize {
    fn from(list: MutationList) -> Self {
        Self::try_from(list.0).unwrap_or(Self::MAX)
    }
}

/// One definition that can reach the name being resolved, with the namespace that holds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FlowCandidate {
    /// The definition.
    pub binding: Position,
    /// The namespace.
    pub scope: Position,
}

/// Whether a binding step gives its name a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Binds {
    /// It does.
    Value,
    /// It does not: an annotation without a value declares the name and binds nothing.
    Nothing,
}

/// Whether a region of the file can run the code in it again after a later binding of the same region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegionRepeat {
    /// It can: a loop's body, a comprehension's body.
    Repeats,
    /// It runs once.
    Once,
}

/// What the reach of a name is worked out with, cleared between files.
#[derive(Clone, Debug, Default)]
pub(crate) struct ReachModel {
    /// The candidates of the name being resolved.
    pub candidates: Vec<FlowCandidate>,
    /// Which list of `mutation_lists` holds the mutations of each name of each namespace.
    pub mutations: HashMap<NamespaceKey, MutationList, BuildHasherDefault<KeyHasher>>,
    /// Every step that binds or unbinds one name of one namespace, in evaluation order; the lists are reused between files.
    pub mutation_lists: Vec<Vec<BindingStep>>,
    /// How many of the lists this file uses.
    pub mutation_count: Count,
}

/// What the flow is interpreted from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FlowSource<'m, 't, 'a> {
    /// The names of the file.
    pub names: &'m NameTable,
    /// The lexical part of the model.
    pub lexical: &'m LexicalModel,
    /// The definitions part of the model.
    pub definitions: &'m DefinitionModel,
    /// The schedule.
    pub schedule: &'m BindingSchedule,
    /// The file.
    pub tree: TypedTree<'t, 'a>,
}
