//! Common preparation owns a model's generation, configuration and lifetime.
use crate::domain::{elements::{
        service::{end_of, engine, engine_mut, real_count},
        types::{Completion, Elements, Tree},
    }, symbols::types::{ByteSlice, Symbols}};
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
            Err(_) => engine_mut(elements).model_preparation.ready = false,
        }
    } else {
        engine_mut(elements).model_preparation.ready = false;
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
    engine_mut(elements).model_preparation.ready = false;
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
    configuration: &str,
    model: &mut M,
    initialize: impl FnOnce(&Tree<'_>, &mut M),
) {
    let native_count = real_count(elements);
    let state = &engine(elements).model_preparation;
    if state.ready && state.native_count == native_count && state.configuration == configuration {
        return;
    }
    self::initialize(elements, source, file, model, initialize);
    let state = &mut engine_mut(elements).model_preparation;
    state.configuration.clear();
    state.configuration.push_str(configuration);
    state.native_count = native_count;
    state.ready = true;
}

struct Phase<'a> {
    completion: Completion<'a>,
    committed: bool,
}
impl Drop for Phase<'_> {
    fn drop(&mut self) {
        if !self.committed {
            engine_mut(self.completion.as_mut().elements)
                .model_preparation
                .ready = false;
        }
    }
}

/// Complete one generation, lending only an engine-initialized model and retaining no stale proof on unwind.
pub(crate) fn complete<M: Default + Send + 'static>(
    completion: Completion<'_>,
    configuration: &str,
    requirement: crate::domain::elements::types::CompletionRequirement,
    initialize: impl FnOnce(&Tree<'_>, &mut M),
    finish: impl FnOnce(&mut Completion<'_>),
    extend: impl FnOnce(&Tree<'_>, Position, &mut M),
) {
    let requires_model =
        requirement == crate::domain::elements::types::CompletionRequirement::PreparedModel;
    let mut phase = Phase {
        completion,
        committed: false,
    };
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
        engine_mut(input.elements).model_preparation.ready = false;
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
        engine_mut(input.elements).model_preparation.ready = true;
    }
    phase.committed = true;
}

/// Lend initialized model data without giving the provider readiness or initialization capabilities.
/// No callback can retain a tree/model borrow across the append phase.
pub fn with_file_model<M: Send + 'static, R>(
    completion: &mut Completion<'_>,
    read: impl FnOnce(&Tree<'_>, &mut Symbols, &M) -> R,
) -> Option<R> {
    let input = completion.as_mut();
    let table = engine(input.elements);
    if !table.model_preparation.ready
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
