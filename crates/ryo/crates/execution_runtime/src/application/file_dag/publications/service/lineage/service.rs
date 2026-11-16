//! Where the rows of a named output and of a bound observation come from, from the program alone: an output run in each file
//! from one file each, one run after the files as its step's rows are, and a binding as the union of its producers' outputs
//! is. A handed-over or published row keeps the file it came from, so a partial answer names the file it lacks.

use crate::application::file_dag::file_pass::service::nodes::service::{
    handed_step, step_coverage_with,
};
use crate::application::file_dag::file_pass::types::PassProgram;
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::observation_schedule::types::{
    ResolvedObservationBinding, ResolvedObservationProducer,
};
use ryo_relations::domain::relations::service::kinds as relation_kinds;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::lineage::service::{of_step, one_file};
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::{Input, ObservationBindingId, StepId, StepScope};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::records::types::ObservationIndex;

/// Where the rows of `rule`'s named output `output` come from: from one file each when it runs in each file, as the rows of
/// the step it names when it runs after the files.
pub(crate) fn output_lineage(
    pass: &PassProgram,
    bindings: &[Vec<ResolvedObservationBinding>],
    rule: RuleIndex,
    output: ObservationIndex,
) -> Lineage {
    output_of(pass, bindings, rule, output, &mut Vec::new())
}

/// Where the rows of `united`'s and `next`'s union come from: a column lies in its row's file only when it does in both.
pub(crate) fn union_lineage(united: Lineage, next: Lineage) -> Lineage {
    let union = Operation::Union {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(1)),
    };
    of_step(&union, &[united, next], &[])
}

/// `output_lineage`, with the rules whose lineage is being found: one reached again proves nothing of its rows.
fn output_of(
    pass: &PassProgram,
    bindings: &[Vec<ResolvedObservationBinding>],
    rule: RuleIndex,
    output: ObservationIndex,
    visiting: &mut Vec<RuleIndex>,
) -> Lineage {
    let Some(declared) = pass.plans.get(rule.0).and_then(|plan| {
        plan.observations
            .get(usize::try_from(output.0).unwrap_or(usize::MAX))
    }) else {
        return Lineage::Mixed;
    };
    if declared.scope == StepScope::PerFile {
        let kinds: Vec<CellKind> = declared.columns.iter().map(|column| column.kind).collect();
        return one_file(&kinds);
    }
    if visiting.contains(&rule) {
        return Lineage::Mixed;
    }
    visiting.push(rule);
    let lineage = match declared.source {
        Input::Step(step) => step_of(pass, bindings, rule, step, visiting),
        Input::BoundObservation(input) => binding_of(pass, bindings, rule, input, visiting),
        Input::Relation(relation) => one_file(relation_kinds(relation)),
    };
    let _: Option<RuleIndex> = visiting.pop();
    lineage
}

/// Where the rows of `rule`'s step `step` come from after the files: a relation's and a handed-over step's from one file each,
/// a bound observation's as its producers' are, and each step on the way passing on its operands' as its operator does.
fn step_of(
    pass: &PassProgram,
    bindings: &[Vec<ResolvedObservationBinding>],
    rule: RuleIndex,
    step: StepId,
    visiting: &mut Vec<RuleIndex>,
) -> Lineage {
    let walked = step_coverage_with(pass, rule, step, |input, scope| match input {
        Input::Relation(relation) => {
            let kinds = relation_kinds(relation);
            Some(Ok((
                kinds.to_vec(),
                CoverageState::Complete,
                one_file(kinds),
            )))
        }
        Input::Step(read) if scope == StepScope::PerFile => {
            Some(Ok(handed_step(pass, rule, read, CoverageState::Complete)))
        }
        Input::Step(_) => None,
        Input::BoundObservation(input) => Some(Ok((
            Vec::new(),
            CoverageState::Complete,
            binding_of(pass, bindings, rule, input, visiting),
        ))),
    });
    match walked {
        Ok((_, _, lineage)) => lineage,
        Err(_) => Lineage::Mixed,
    }
}

/// Where the rows of `rule`'s binding `input` come from: the union of its producers' outputs, a binding no producer feeds from
/// no one file.
fn binding_of(
    pass: &PassProgram,
    bindings: &[Vec<ResolvedObservationBinding>],
    rule: RuleIndex,
    input: ObservationBindingId,
    visiting: &mut Vec<RuleIndex>,
) -> Lineage {
    let Some(binding) = bindings
        .get(rule.0)
        .and_then(|declared| declared.iter().find(|binding| binding.input == input))
    else {
        return Lineage::Mixed;
    };
    let mut members = binding.producers.iter().map(|producer| match producer {
        ResolvedObservationProducer::Output { rule, output } => {
            output_of(pass, bindings, *rule, *output, visiting)
        }
        ResolvedObservationProducer::NoQuery { .. } => Lineage::Mixed,
    });
    match members.next() {
        Some(first) => members.fold(first, union_lineage),
        None => Lineage::Mixed,
    }
}
