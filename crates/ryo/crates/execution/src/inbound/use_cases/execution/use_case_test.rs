//! A request execution begins over a prepared program and finishes.
use super::fixtures::requests::{Authored, Preparation};
use super::{interface::ExecutionUseCase, use_case::ExecutionUseCaseImpl};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort;

/// What a begun request answered.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Finished,
}
type Input = ();
type Value = Outcome;
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName(stringify!(an_empty_request_begins_and_finishes)),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Outcome::Finished),
}];
fn subject((): Input) -> Result<Value, Failure> {
    let prepare = Preparation::real();
    let prepared = prepare.prepare(Authored {
        plans: vec![],
        bindings: vec![],
        rules: vec![],
        digests: vec![],
        hosted: vec![],
        observations: ObservationIdentity::Independent,
    })?;
    let request = ExecutionUseCaseImpl::new().execution(&prepared, &[], None.into());
    ProgramRequestExecutionPort::finish(request);
    Ok(Outcome::Finished)
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
mod conformance_emptied;

#[cfg(test)]
mod conformance_boundary;

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

#[cfg(test)]
mod interpreter;
