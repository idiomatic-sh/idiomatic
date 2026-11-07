//! Common preparation owns a model's generation, configuration and lifetime.
use crate::domain::{
    elements::{
        service::{end_of, engine, engine_mut, real_count},
        types::{Completion, Elements, ModelReadiness, Tree},
    },
    symbols::types::{ByteSlice, Symbols},
};
use ryo_relations::domain::relations::types::{FileId, Position};

/// Take the existing worker room, invalidating readiness when its concrete model differs.
#[expect(
    clippy::unnecessary_box_returns,
    reason = "the existing worker-owned model box is reused between files"
)]
pub(crate) fn take_model<M: Default + Send + 'static>(elements: &mut Elements) -> Box<M> {
    if let Some(kept) = engine_mut(elements).scratch.0.take() {
        match kept.downcast::<M>() {
            Ok(model) => return model,
            Err(_) => engine_mut(elements).model_preparation.ready = ModelReadiness::Stale,
        }
    } else {
        engine_mut(elements).model_preparation.ready = ModelReadiness::Stale;
    }
    Box::default()
}

/// Return exactly the same model allocation to its engine owner.
pub(crate) fn keep_model<M: Send + 'static>(elements: &mut Elements, model: Box<M>) {
    engine_mut(elements).scratch.0 = Some(model);
}

/// Preserve ordinary providers' per-walk initialization without publishing completion readiness.
pub(crate) fn initialize<M>(
    elements: &mut Elements,
    source: ByteSlice<'_>,
    file: FileId,
    model: &mut M,
    initialize: impl FnOnce(&Tree<'_>, &mut M),
) {
    engine_mut(elements).model_preparation.ready = ModelReadiness::Stale;
    initialize(
        &Tree {
            elements,
            source,
            file,
        },
        model,
    );
}

/// The engine supplies the existing provider initialization; completion callbacks cannot mark readiness.
pub(crate) fn prepare<M>(
    elements: &mut Elements,
    source: ByteSlice<'_>,
    file: FileId,
    configuration: ByteSlice<'_>,
    model: &mut M,
    initialize: impl FnOnce(&Tree<'_>, &mut M),
) {
    let native_count = real_count(elements);
    let state = &engine(elements).model_preparation;
    if state.ready == ModelReadiness::Ready
        && state.native_count == native_count
        && state.configuration.0 == configuration.0
    {
        return;
    }
    self::initialize(elements, source, file, model, initialize);
    let state = &mut engine_mut(elements).model_preparation;
    state.configuration.0.clear();
    state.configuration.0.extend_from_slice(configuration.0);
    state.native_count = native_count;
    state.ready = ModelReadiness::Ready;
}

/// Whether a completion phase reached its end.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Commit {
    /// The phase has not ended: a drop now is an unwind.
    Pending,
    /// The phase ended.
    Done,
}

struct Phase<'a> {
    completion: Completion<'a>,
    committed: Commit,
}
impl Drop for Phase<'_> {
    fn drop(&mut self) {
        if self.committed == Commit::Pending {
            engine_mut(self.completion.as_mut().elements)
                .model_preparation
                .ready = ModelReadiness::Stale;
        }
    }
}

/// Complete one generation, lending only an engine-initialized model and retaining no stale proof on unwind.
pub(crate) fn complete<M: Default + Send + 'static>(
    completion: Completion<'_>,
    configuration: ByteSlice<'_>,
    requirement: crate::domain::elements::types::CompletionRequirement,
    initialize: impl FnOnce(&Tree<'_>, &mut M),
    finish: impl FnOnce(&mut Completion<'_>),
    extend: impl FnOnce(&Tree<'_>, Position, &mut M),
) {
    let requires_model =
        requirement == crate::domain::elements::types::CompletionRequirement::PreparedModel;
    let mut phase = Phase {
        completion,
        committed: Commit::Pending,
    };
    phase.completion.as_mut().requirement = requirement;
    if requires_model {
        let input = phase.completion.as_mut();
        let mut model = take_model::<M>(input.elements);
        prepare(
            input.elements,
            input.source,
            input.file,
            configuration,
            &mut *model,
            initialize,
        );
        keep_model(input.elements, model);
    }
    let from = end_of(phase.completion.as_mut().elements);
    finish(&mut phase.completion);
    if requires_model && end_of(phase.completion.as_mut().elements) > from {
        let input = phase.completion.as_mut();
        let mut model = take_model::<M>(input.elements);
        engine_mut(input.elements).model_preparation.ready = ModelReadiness::Stale;
        extend(
            &Tree {
                elements: input.elements,
                source: input.source,
                file: input.file,
            },
            from,
            &mut *model,
        );
        keep_model(input.elements, model);
        engine_mut(input.elements).model_preparation.ready = ModelReadiness::Ready;
    }
    phase.committed = Commit::Done;
}

/// What this completion was given: `PreparedModel` only when the language requires a model and the request's demand reads what
/// completion adds (a reader of expanded elements); otherwise `Independent`, and no model is lent.
pub fn completion_requirement(
    completion: &mut Completion<'_>,
) -> crate::domain::elements::types::CompletionRequirement {
    completion.as_mut().requirement
}

/// Lend initialized model data without giving the provider readiness or initialization capabilities.
/// No callback can retain a tree/model borrow across the append phase.
pub fn with_file_model<M: Send + 'static, R>(
    completion: &mut Completion<'_>,
    read: impl FnOnce(&Tree<'_>, &mut Symbols, &M) -> R,
) -> Option<R> {
    let input = completion.as_mut();
    let table = engine(input.elements);
    if table.model_preparation.ready == ModelReadiness::Stale
        || table.model_preparation.native_count != real_count(input.elements)
    {
        return None;
    }
    let model = table.scratch.0.as_ref()?.downcast_ref::<M>()?;
    Some(read(
        &Tree {
            elements: input.elements,
            source: input.source,
            file: input.file,
        },
        input.symbols,
        model,
    ))
}
