//! The catalogs hold: every kind of this stage lowers once, and every kernel is run by some lowering of the case program;
//! each check is shown failing on a planted counter-example.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::interface::Reach;
use crate::domain::physical_plan::fixtures::{Step, case_program};
use crate::domain::physical_plan::service::{Kernel, REWRITES, lowered, plan_with, stage_of_kind};
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::{
    LogicalNode, Lowering, NodeBindings, PlanInput, PlanInputList, RuleOrdinal, ServedAt,
};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::OperationKind;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    KindsLower,
    KindLeftUnlowered,
    KernelsReached,
    KernelNoLoweringRuns,
}
/// What a check found broken.
#[derive(Debug, PartialEq)]
enum Breach {
    /// A kind of this stage the case program has no lowered node of.
    Unlowered(OperationKind),
    /// A kernel no lowering of the case program runs.
    Unreached(Kernel),
    /// The case program was refused.
    Refused(PortableOperatorFault),
}
type Input = Scenario;
/// How many kinds or kernels the check held for.
type Value = usize;
type Failure = Breach;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_kind_lowers_once"),
        word: Word::Succeeds,
        input: || Scenario::KindsLower,
        expect: Expect::Value(9),
    },
    Case {
        name: CaseName("a_kind_left_unlowered_fails_every_kind_lowers_once"),
        word: Word::Refuses,
        input: || Scenario::KindLeftUnlowered,
        expect: Expect::Error(Breach::Unlowered(OperationKind::Extend)),
    },
    Case {
        name: CaseName("every_kernel_is_reached"),
        word: Word::Succeeds,
        input: || Scenario::KernelsReached,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName("a_kernel_no_lowering_runs_fails_every_kernel_is_reached"),
        word: Word::Refuses,
        input: || Scenario::KernelNoLoweringRuns,
        expect: Expect::Error(Breach::Unreached(Kernel::Group)),
    },
];

/// The lowering function a check reads: the planner's, or a planted one.
type Lower = fn(&Operation, &[PlanInput], &[ParameterBinding]) -> Lowering;

/// The planted counter-example of a lowering: the planner's, with an extension left unlowered.
fn extension_unlowered(
    operation: &Operation,
    inputs: &[PlanInput],
    bindings: &[ParameterBinding],
) -> Lowering {
    if let Operation::Extend { .. } = operation {
        Lowering::Unlowered
    } else {
        lowered(operation, inputs, bindings)
    }
}

/// Every kind of this stage has a node of the case program `lower` lowers; every other kind is a later stage's.
fn kinds_lower(lower: Lower) -> Result<Value, Failure> {
    let steps = case_program();
    let mut held = 0;
    for kind in OperationKind::ALL {
        if stage_of_kind(*kind) > ServedAt(1) {
            continue;
        }
        if !steps.iter().any(|step| {
            kind_of(&step.operation) == *kind
                && lower(&step.operation, &step.inputs, &[]) != Lowering::Unlowered
        }) {
            return Err(Breach::Unlowered(*kind));
        }
        held += 1;
    }
    Ok(held)
}

/// Every kernel is run by a node of `steps` planned with every rewrite.
fn kernels_reached(steps: &[Step]) -> Result<Value, Failure> {
    let mut program = LogicalProgram::new();
    for (at, step) in steps.iter().enumerate() {
        let id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
        program.emit(RuleOrdinal(u32::try_from(at).unwrap_or(u32::MAX)), id);
    }
    let plan = plan_with(&program, REWRITES).map_err(Breach::Refused)?;
    for kernel in Kernel::ALL {
        if !plan
            .nodes
            .iter()
            .any(|node| kernel.item().reached_by(&node.lowering) == Reach::Runs)
        {
            return Err(Breach::Unreached(*kernel));
        }
    }
    Ok(Kernel::ALL.len())
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::KindsLower => kinds_lower(lowered),
        Scenario::KindLeftUnlowered => kinds_lower(extension_unlowered),
        Scenario::KernelsReached => kernels_reached(&case_program()),
        Scenario::KernelNoLoweringRuns => {
            let steps: Vec<Step> = case_program()
                .into_iter()
                .filter(|step| !matches!(step.operation, Operation::GroupCount { .. }))
                .collect();
            kernels_reached(&steps)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
