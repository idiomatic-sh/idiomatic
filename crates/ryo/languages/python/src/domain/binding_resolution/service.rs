//! Schedule-order resolution (Spike ryo-fmx3): run the file's schedule in order, keep the latest event of each (namespace, name),
//! and at each read or deletion walk Python's lookup path (Language Reference §4.2.2, §7.12, §7.13; PEP 695), one row per namespace
//! that holds an event of the name. Alternatives are linearised in schedule order; no linter policy is applied here.
use super::types::BindingResolutionScratch;
use crate::domain::{
    binding_analysis::service::indexed,
    model::types::{BindingAnalysisIndex, BindingAnalysisState, Declarations},
};
use ryo::languages::{Symbols, Tree};
use std::cell::Ref;

/// The file's resolution, made on the first call for the file; `None` when the binding analysis is incomplete.
pub(crate) fn resolved<'a>(
    tree: &Tree<'_>,
    model: &'a Declarations,
    symbols: &mut Symbols,
) -> Option<Ref<'a, BindingResolutionScratch>> {
    let index = indexed(tree, model, symbols)?;
    if model.binding_resolution.borrow().state == BindingAnalysisState::Fresh {
        let mut out = model.binding_resolution.borrow_mut();
        resolve(tree, model, &index, &mut out);
        out.state = BindingAnalysisState::Complete;
    }
    drop(index);
    Some(model.binding_resolution.borrow())
}

/// Forget the current file's resolution while keeping every buffer.
pub(crate) fn clear(out: &mut BindingResolutionScratch) {
    out.state = BindingAnalysisState::Fresh;
    out.resolutions.clear();
    out.latest.clear();
    out.parameters.clear();
    out.lazy.clear();
    out.order.clear();
}

/// S1 stub (Spike ryo-fmx3): no rows until S2 builds the resolver.
const fn resolve(
    _tree: &Tree<'_>,
    _model: &Declarations,
    _index: &BindingAnalysisIndex,
    _out: &mut BindingResolutionScratch,
) {
}
