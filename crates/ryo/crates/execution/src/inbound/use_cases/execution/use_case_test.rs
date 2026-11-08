//! A request execution begins over a prepared program, and its file owner reuses only recognized stable sources.
use super::fixtures::requests::{Authored, Preparation};
use super::{interface::ExecutionUseCase, use_case::ExecutionUseCaseImpl};
use crate::application::file_dag::program::file_session::interface::FileExecution;
use crate::application::file_dag::program::file_session::types::{FileInput, SourceLanguage};
use crate::application::file_dag::program::request_execution::interface::ExecutionRequest;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operating::errors::OperatingError;
use crate::domain::prepared_request::types::ObservationIdentity;
use crate::outbound::facts::types::ResidentStability;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use std::sync::Arc;

/// What a begun request is asked to do.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    Empty,
    File(SourceLanguage, ResidentStability),
}
/// What the begun request answered.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Finished,
    Reuse(ryo_execution_store::application::file_dag::relations::types::ReuseEligibility),
}
type Input = Scenario;
type Value = Outcome;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(an_empty_request_begins_and_finishes)),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(Outcome::Finished),
    },
    Case {
        name: CaseName(stringify!(a_recognized_stable_source_is_reused)),
        word: Word::Succeeds,
        input: || Scenario::File(SourceLanguage::Recognized, ResidentStability::Stable),
        expect: Expect::Value(Outcome::Reuse(ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible)),
    },
    Case {
        name: CaseName(stringify!(an_unstable_source_is_not_reused)),
        word: Word::Succeeds,
        input: || Scenario::File(SourceLanguage::Recognized, ResidentStability::Unstable),
        expect: Expect::Value(Outcome::Reuse(ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Ineligible)),
    },
    Case {
        name: CaseName(stringify!(an_unrecognized_source_is_not_reused)),
        word: Word::Succeeds,
        input: || Scenario::File(SourceLanguage::Unrecognized, ResidentStability::Stable),
        expect: Expect::Value(Outcome::Reuse(ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Ineligible)),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    let prepare = Preparation::real();
    let prepared = prepare.prepare(Authored { plans: vec![],
            bindings: vec![], rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    })?;
    match input {
        Scenario::Empty => {
            let request = ExecutionUseCaseImpl::new().execution(&prepared, &[]);
            ExecutionRequest::finish(request);
            Ok(Outcome::Finished)
        }
        Scenario::File(language, resident_stability) => {
            let corpus = vec![CorpusInput::Available(CorpusEntry {
                path: ryo_language::domain::files::types::FilePath("empty.py".into()),
                executable: ExecutablePermission::Unknown,
                size: FileSize(0),
                modified: Nanoseconds(0),
            })];
            let request = ExecutionUseCaseImpl::new().execution(&prepared, &corpus);
            let elements = ryo_language::domain::elements::types::Elements::default();
            let read = |_| Err(OperatingError::NotRead);
            let mut symbols = ryo_language::domain::symbols::types::Symbols::default();
            let file = request.open_file(
                FileInput {
                    language,
                    resident_stability,
                    file: ryo_relations::domain::relations::types::FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[],
                    source: ryo_values::domain::files::types::SourceView(&[]),
                    elements: &elements,
                    read: &read,
                    semantic: crate::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: crate::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )?;
            let eligibility = file.reuse_eligible();
            file.finish()?;
            Ok(Outcome::Reuse(eligibility))
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
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
mod closure_partitions;

#[cfg(test)]
mod graph;
#[cfg(test)]
mod span_join;

#[cfg(test)]
mod kinds;

#[cfg(test)]
mod axis;

#[cfg(test)]
mod axis_tree;

#[cfg(test)]
mod capability;

#[cfg(test)]
mod axis_graph;

#[cfg(test)]
mod conformance;

#[cfg(test)]
mod conformance_driver;

#[cfg(test)]
mod conformance_boundary;
#[cfg(test)]
mod conformance_scale;


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
mod contract_timing;

#[cfg(test)]
mod admission_order;

#[cfg(test)]
mod admission_order_precedence;

#[cfg(test)]
mod errored_rule;
