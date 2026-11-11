//! The support of the `identity` cases.
use super::super::super::super::fixtures::operations::{corpus::compile_program, heads::FUNCTION};
use ryo_execution_domain::domain::executable_program::{
    fusion::types::Fusibility,
    types::{LogicalInstruction, executable_program::value::ExecutableProgram},
};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Input, Operand, PlanComparison, QueryLiteral, RepeatedRows, StepId, StepScope,
        Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
};

pub(super) fn equal(column: ColumnIndex) -> PlanComparison {
    PlanComparison {
        test: Test::Equal,
        left: Operand::Column(column),
        right: Operand::Column(column),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    }
}

pub(super) fn plan_of(steps: Vec<Operation>, emit: u32) -> Plan {
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

pub(super) fn producer() -> Operation {
    Operation::Select {
        from: Input::Relation(FUNCTION),
        comparisons: vec![equal(ColumnIndex(1))],
        kept: vec![],
    }
}

pub(super) fn keep(step: u32) -> Operation {
    Operation::Project {
        from: Input::Step(StepId(step)),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Kept,
    }
}

pub(super) fn extended(step: u32, number: i64) -> Operation {
    Operation::Extend {
        from: Input::Step(StepId(step)),
        values: vec![Operand::Literal(QueryLiteral::Number(Integer(number)))],
    }
}

pub(super) fn logical(program: &ExecutableProgram, rule: usize, step: usize) -> LogicalInstruction {
    program.rules()[rule].original_steps[step]
        .as_ref()
        .unwrap()
        .logical
}

pub(super) fn class(program: &ExecutableProgram, step: usize) -> Fusibility {
    program.fusibility(logical(program, 0, step)).unwrap()
}

pub(super) fn region_len(program: &ExecutableProgram, step: usize) -> usize {
    let region = program.logical_regions()[logical(program, 0, step).0];
    program.regions()[region.0].instructions().len()
}

pub(super) fn compile(plan: Plan) -> ExecutableProgram {
    compile_program(&[plan], &[vec![]])
}

pub(super) fn emit_region(program: &ExecutableProgram, rule: usize) -> usize {
    program.rules()[rule].emit.as_ref().unwrap().0
}
