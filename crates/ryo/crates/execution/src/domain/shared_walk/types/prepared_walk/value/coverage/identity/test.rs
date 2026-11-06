//! Fusion across an owner-issued identity is refused; equal Extends of two rules compute once.
use super::super::fixtures::operations::{FUNCTION, compile_program};
use crate::domain::executable_program::{
    fusion::types::{Barrier, Fusibility, Ineligible},
    types::{ExecutableProgram, LogicalInstruction},
};
use ryo_relations::domain::relations::{service::kinds, types::Integer};
use ryo_values::domain::{
    observations::types::{ObservationColumn, ObservationColumnName, ObservationName, PlanObservation},
    plan_parts::types::{ColumnIndex, Comparison, Input, JoinColumn, Operand, QueryLiteral, RepeatedRows, Side, StepId, Test, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
};
use std::collections::HashSet;

fn equal(column: ColumnIndex) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(column),
        right: Operand::Column(column),
        origin: None,
    }
}
fn plan_of(steps: Vec<Operation>, emit: u32) -> Plan {
    Plan {
        observation_inputs: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        observations: vec![],
        steps: steps
            .into_iter()
            .map(|operation| PlanStep {
                operation,
                scope: StepScope::PerFile,
            })
            .collect(),
        emit: Emit {
            step: StepId(emit),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
    }
}
fn producer() -> Operation {
    Operation::Select {
        from: Input::Relation(FUNCTION),
        comparisons: vec![equal(ColumnIndex(1))],
        kept: vec![],
    }
}
fn keep(step: u32) -> Operation {
    Operation::Project {
        from: Input::Step(StepId(step)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Kept,
    }
}
fn extended(step: u32, number: i64) -> Operation {
    Operation::Extend {
        from: Input::Step(StepId(step)),
        values: vec![Operand::Literal(QueryLiteral::Number(Integer(number)))],
    }
}
fn logical(program: &ExecutableProgram, rule: usize, step: usize) -> LogicalInstruction {
    program.rules()[rule].original_steps[step]
        .as_ref()
        .unwrap()
        .logical
}
fn class(program: &ExecutableProgram, step: usize) -> Fusibility {
    program.fusibility(logical(program, 0, step)).unwrap()
}
fn region_len(program: &ExecutableProgram, step: usize) -> usize {
    let region = program.logical_regions()[logical(program, 0, step).0];
    program.regions()[region.0].instructions.len()
}
fn compile(plan: Plan) -> ExecutableProgram {
    compile_program(&[plan], &[vec![]])
}

#[test]
fn a_producer_read_once_by_a_pure_projection_is_fused() {
    let program = compile(plan_of(vec![producer(), keep(0)], 1));
    assert_eq!(class(&program, 0), Fusibility::Excluded(Ineligible::Source));
    assert_eq!(class(&program, 1), Fusibility::Fusable);
    assert_eq!(region_len(&program, 1), 2);
}

#[test]
fn a_handed_over_root_is_not_fused_into() {
    let program = compile(plan_of(vec![producer(), keep(0)], 0));
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Handover));
    assert_eq!(region_len(&program, 1), 1);
}

#[test]
fn a_frontier_root_is_not_fused_into() {
    let mut plan = plan_of(vec![producer(), keep(0)], 1);
    plan.frontier.push(Input::Step(StepId(0)));
    let program = compile(plan);
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Frontier));
    assert_eq!(region_len(&program, 1), 1);
}

#[test]
fn an_observation_source_is_not_fused_into() {
    let mut plan = plan_of(vec![producer(), keep(0)], 1);
    plan.observations.push(PlanObservation {
        name: ObservationName("picked".into()),
        source: Input::Step(StepId(0)),
        columns: kinds(FUNCTION)
            .iter()
            .enumerate()
            .map(|(at, kind)| ObservationColumn {
                name: ObservationColumnName(format!("column-{at}").into()),
                kind: *kind,
            })
            .collect(),
        scope: StepScope::PerFile,
    });
    let program = compile(plan);
    assert_eq!(
        class(&program, 1),
        Fusibility::Barrier(Barrier::Observation)
    );
    assert_eq!(region_len(&program, 1), 1);
}

#[test]
fn a_derived_acquisition_source_is_not_fused_into() {
    let join = Operation::Join {
        left: Input::Relation(FUNCTION),
        right: Input::Step(StepId(0)),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![
            JoinColumn {
                side: Side::Left,
                column: ColumnIndex(0),
            },
            JoinColumn {
                side: Side::Right,
                column: ColumnIndex(1),
            },
        ],
        filter: vec![],
        view: vec![],
        reach: None,
    };
    let program = compile(plan_of(vec![producer(), keep(0), join], 2));
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Derived));
    assert_eq!(region_len(&program, 1), 1);
}

#[test]
fn a_shared_consumer_keeps_its_producer_publication() {
    let other = Operation::Project {
        from: Input::Step(StepId(0)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let program = compile(plan_of(vec![producer(), keep(0), other], 1));
    assert_eq!(
        class(&program, 1),
        Fusibility::Barrier(Barrier::SharedConsumer)
    );
    assert_eq!(
        class(&program, 2),
        Fusibility::Excluded(Ineligible::Reshapes)
    );
    assert_eq!(region_len(&program, 1), 1);
}

#[test]
fn a_partition_boundary_is_not_fused_across() {
    let mut plan = plan_of(vec![producer(), keep(0)], 1);
    plan.steps[1].scope = StepScope::CrossFile;
    plan.frontier.push(Input::Step(StepId(0)));
    let program = compile(plan);
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Partition));
    assert_eq!(region_len(&program, 1), 1);
}

#[test]
fn an_absence_sensitive_producer_publishes_before_a_continuation() {
    let antijoin = Operation::Antijoin {
        left: Input::Relation(FUNCTION),
        right: Input::Relation(FUNCTION),
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
    };
    let program = compile(plan_of(vec![antijoin, keep(0)], 1));
    assert_eq!(class(&program, 1), Fusibility::Barrier(Barrier::Fault));
    assert_eq!(region_len(&program, 1), 1);
}

fn emit_region(program: &ExecutableProgram, rule: usize) -> usize {
    program.rules()[rule].emit.as_ref().unwrap().0
}

#[test]
fn equivalent_extends_of_two_rules_share_one_region() {
    let program = compile_program(
        &[
            plan_of(vec![producer(), extended(0, 7)], 1),
            plan_of(vec![producer(), extended(0, 7)], 1),
        ],
        &[vec![], vec![]],
    );
    assert_eq!(emit_region(&program, 0), emit_region(&program, 1));
    let shared = &program.regions()[emit_region(&program, 0)];
    assert_eq!(shared.consumers, 2, "both rules read the one publication");
    assert_eq!(program.regions().len(), 2, "one producer, one extend");
    let producer = program.logical_regions()[logical(&program, 0, 0).0];
    assert_eq!(program.regions()[producer.0].consumers, 1);
    // Each rule still names an instruction its region contains.
    for rule in 0..2 {
        let at = logical(&program, rule, 1);
        let region = program.logical_regions()[at.0];
        assert!(
            program.regions()[region.0]
                .instructions
                .iter()
                .any(|instruction| instruction.logical == at)
        );
    }
}

#[test]
fn extends_that_differ_in_operand_or_input_keep_their_own_regions() {
    let other_producer = Operation::Select {
        from: Input::Relation(FUNCTION),
        comparisons: vec![equal(ColumnIndex(0))],
        kept: vec![],
    };
    let program = compile_program(
        &[
            plan_of(vec![producer(), extended(0, 7)], 1),
            plan_of(vec![producer(), extended(0, 8)], 1),
            plan_of(vec![other_producer, extended(0, 7)], 1),
        ],
        &[vec![], vec![], vec![]],
    );
    let regions = (0..3)
        .map(|rule| emit_region(&program, rule))
        .collect::<HashSet<_>>();
    assert_eq!(
        regions.len(),
        3,
        "a different literal or input is never shared"
    );
}
