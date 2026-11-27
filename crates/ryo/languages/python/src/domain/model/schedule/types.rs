//! The types of the model's `schedule` part: plain data, public fields and derives, with no constructor and no inherent impl.
//! The schedule itself is the `model` concept's `BindingSchedule`.

use crate::domain::model::definitions::types::DefinitionModel;
use crate::domain::model::lexical::types::{LexicalModel, TypedTree};
use crate::domain::model::types::BindingAnnotationTiming;

/// What the schedule is built from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ScheduleSource<'m, 't, 'a> {
    /// The lexical part of the model.
    pub lexical: &'m LexicalModel,
    /// The definitions part of the model.
    pub(crate) definitions: &'m DefinitionModel,
    /// The file.
    pub tree: TypedTree<'t, 'a>,
    /// When the file's annotations are evaluated.
    pub(crate) timing: BindingAnnotationTiming,
}
