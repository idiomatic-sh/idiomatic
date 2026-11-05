//! Public preparation must reject structurally mismatched request inputs.
use super::{interface::ExecutionUseCase, types::*, use_case::ExecutionUseCaseImpl};
use std::sync::Arc;
#[test]
fn rejects_binding_population_without_a_rule() {
    let engine = ExecutionUseCaseImpl::new(
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
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
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
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
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
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
            true,
        ),
        (
            SourceLanguage::Recognized,
            facts::ResidentStability::Unstable,
            false,
        ),
        (
            SourceLanguage::Unrecognized,
            facts::ResidentStability::Stable,
            false,
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
mod rowsets_test;

#[cfg(test)]
mod ordered_match_test;

#[cfg(test)]
mod union_test;

#[cfg(test)]
mod rank_test;
#[cfg(test)]
mod replace_test;
#[cfg(test)]
mod rows_test;

#[cfg(test)]
mod text_test;

#[cfg(test)]
mod cycles_test;
#[cfg(test)]
mod digest_test;

#[cfg(test)]
mod provider_test;

#[cfg(test)]
mod tree_join_test;

#[cfg(test)]
mod select_test;

#[cfg(test)]
mod extend_test;

#[cfg(test)]
mod closure_test;

#[cfg(test)]
mod graph_test;
#[cfg(test)]
mod span_join_test;

#[cfg(test)]
mod kinds_test;

#[cfg(test)]
mod axis_test;

#[cfg(test)]
mod capability_test;

#[cfg(test)]
mod axis_graph_test;

#[cfg(test)]
mod conformance_test;

#[cfg(test)]
mod conformance_boundary_test;
#[cfg(test)]
mod conformance_scale_test;

#[cfg(test)]
mod selection_view_test;

#[cfg(test)]
mod dispatch_test;

#[cfg(test)]
mod schema_test;

#[cfg(test)]
mod binding_schema_test;

#[cfg(test)]
mod traversal_schema_test;

#[cfg(test)]
mod prepared_matching_test;

#[cfg(test)]
mod project_input_test;
