//! Public preparation must reject structurally mismatched request inputs.
use super::{interface::ExecutionUseCase, types::*, use_case::ExecutionUseCaseImpl};
use crate::application::file_dag::program::facade_runtime::types::ReuseEligibility;
use std::sync::Arc;
#[test]
fn rejects_binding_population_without_a_rule() {
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
        ),
        Arc::new(crate::outbound::derived::factory::semantics()),
    );
    let request = PreparedRequest {
        plans: vec![],
        bindings: vec![vec![]],
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    };
    assert!(matches!(
        engine.prepare(request),
        Err(OperatingError::ObservationConfiguration(_))
    ));
}
#[test]
fn empty_request_can_begin_and_finish_without_acquiring_inputs() {
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
        ),
        Arc::new(crate::outbound::derived::factory::semantics()),
    );
    let request = PreparedRequest {
        plans: vec![],
        bindings: vec![],
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    };
    let prepared = engine
        .prepare(request)
        .expect("empty request is legitimate");
    let run = engine.begin(&prepared, &[]);
    super::interface::ExecutionRequest::finish(run);
}

#[test]
fn file_owner_reuses_only_recognized_stable_sources() {
    use super::interface::{ExecutionRequest, FileExecution};
    use ryo_values::domain::files::types::{
        CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
    };
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
        ),
        Arc::new(crate::outbound::derived::factory::semantics()),
    );
    let prepared = engine
        .prepare(PreparedRequest {
            plans: vec![],
            bindings: vec![],
            rules: vec![],
            digests: vec![],
            hosted: vec![],
            observations: ObservationIdentity::Independent,
        })
        .expect("valid empty program");
    let corpus = vec![CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("empty.py".into()),
        executable: ExecutablePermission::Unknown,
        size: FileSize(0),
        modified: Nanoseconds(0),
    })];
    let request = engine.begin(&prepared, &corpus);
    let elements = ryo_language::domain::elements::types::Elements::default();
    let read = |_| Err(OperatingError::NotRead);
    for (language, resident_stability, expected) in [
        (
            SourceLanguage::Recognized,
            facts::ResidentStability::Stable,
            ReuseEligibility::Eligible,
        ),
        (
            SourceLanguage::Recognized,
            facts::ResidentStability::Unstable,
            ReuseEligibility::Ineligible,
        ),
        (
            SourceLanguage::Unrecognized,
            facts::ResidentStability::Stable,
            ReuseEligibility::Ineligible,
        ),
    ] {
        let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
        let file = request
            .open_file(
                FileInput {
                    language,
                    resident_stability,
                    file: ryo_relations::domain::relations::types::FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[],
                    source: &[],
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .expect("normal inbound file owner");
        assert_eq!(file.inner.source_reuse_eligible(), expected);
        file.finish().expect("finish file owner");
    }
}

#[cfg(test)]
mod rowsets;

#[cfg(test)]
mod ordered_match;

#[cfg(test)]
mod union;

#[cfg(test)]
mod rank;
#[cfg(test)]
mod replace;
#[cfg(test)]
mod rows;

#[cfg(test)]
mod text;

#[cfg(test)]
mod cycles;
#[cfg(test)]
mod digest;

#[cfg(test)]
mod provider;

#[cfg(test)]
mod tree_join;

#[cfg(test)]
mod select;

#[cfg(test)]
mod extend;

#[cfg(test)]
mod closure;

#[cfg(test)]
mod graph;
#[cfg(test)]
mod span_join;

#[cfg(test)]
mod kinds;

#[cfg(test)]
mod axis;

#[cfg(test)]
mod capability;

#[cfg(test)]
mod axis_graph;

#[cfg(test)]
mod conformance;

#[cfg(test)]
mod conformance_boundary;
#[cfg(test)]
mod conformance_scale;

#[cfg(test)]
mod selection_view;

#[cfg(test)]
mod dispatch;

#[cfg(test)]
mod schema;

#[cfg(test)]
mod binding_schema;

#[cfg(test)]
mod traversal_schema;

#[cfg(test)]
mod prepared_matching;

#[cfg(test)]
mod project_input;

#[cfg(test)]
mod selection_cache;

#[cfg(test)]
mod contract_timing;

#[cfg(test)]
mod admission_order;

#[cfg(test)]
mod errored_rule;
