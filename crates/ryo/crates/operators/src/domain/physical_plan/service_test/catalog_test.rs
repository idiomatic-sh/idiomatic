//! The catalogs hold: every kind of a served stage lowers once, and every kernel is run by some lowering of the case program;
//! each check is shown failing on a planted counter-example.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::collections::kernels::interface::Reach;
use crate::domain::physical_plan::fixtures::{Step, text_program};
use crate::domain::physical_plan::service::lowering::service::lowered;
use crate::domain::physical_plan::service::{Kernel, REWRITES, plan_with};
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, PlanInput, PlanInputList, PlanNodeId,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId, StepScope};
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
    /// A kind of a served stage the case program has no lowered node of.
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
        expect: Expect::Value(27),
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
        expect: Expect::Value(22),
    },
    Case {
        name: CaseName("a_kernel_no_lowering_runs_fails_every_kernel_is_reached"),
        word: Word::Refuses,
        input: || Scenario::KernelNoLoweringRuns,
        expect: Expect::Error(Breach::Unreached(Kernel::Group)),
    },
];

/// The lowering function a check reads: the planner's, or a planted one.
type Lower = fn(&Operation, &[PlanInput], &[ParameterBinding]) -> Option<Lowering>;

/// The planted counter-example of a lowering: the planner's, with an extension left unlowered.
fn extension_unlowered(
    operation: &Operation,
    inputs: &[PlanInput],
    bindings: &[ParameterBinding],
) -> Option<Lowering> {
    if let Operation::Extend { .. } = operation {
        None
    } else {
        lowered(operation, inputs, bindings)
    }
}

/// The case program with a scan of the host's rows and a scan of a table parameter's rows: the kinds that read no input.
fn with_bound_scans(mut steps: Vec<Step>) -> Vec<Step> {
    steps.push(Step {
        operation: Operation::Hosted {
            relation: ryo_values::domain::plan_parts::types::HostedNumber(0),
        },
        inputs: Vec::new(),
    });
    steps.push(Step {
        operation: Operation::Rows {
            parameter: ryo_values::domain::plan_parts::types::ParameterId(0),
            keys: Vec::new(),
        },
        inputs: Vec::new(),
    });
    steps
}

/// The program the catalogs are checked over: the case program with its bound scans, and a worklist over node 12's one row,
/// a count, for its roots, edges and lattice.
fn catalog_program() -> Vec<Step> {
    let mut steps = with_bound_scans(text_program());
    let count = || ryo_values::domain::plan_parts::types::Input::Step(StepId(12));
    let first = ColumnIndex(0);
    steps.push(Step {
        operation: Operation::Worklist {
            from: count(),
            edges: count(),
            lattice: count(),
            root: first,
            state: first,
            source: first,
            destination: first,
            order: first,
            before: first,
            after: first,
            lower: first,
            upper: first,
            root_keys: Vec::new(),
            edge_keys: Vec::new(),
        },
        inputs: vec![PlanInput::Node(PlanNodeId(12)); 3],
    });
    steps.push(Step {
        operation: Operation::BandJoin {
            left: count(),
            right: count(),
            left_on: vec![first],
            right_on: vec![first],
            band: vec![ryo_values::domain::plan_parts::types::BandEdge {
                left: first,
                test: ryo_values::domain::plan_parts::types::BandTest::Below,
                right: first,
            }],
            answer: ryo_values::domain::plan_parts::types::BandAnswer::Nearest,
            output: Vec::new(),
            filter: Vec::new(),
            view: Vec::new(),
            fan: ryo_values::domain::plan_parts::types::KeyFan::Shared,
        },
        inputs: vec![PlanInput::Node(PlanNodeId(12)); 2],
    });
    steps
}

/// Every kind has a node of the case program `lower` lowers.
fn kinds_lower(lower: Lower) -> Result<Value, Failure> {
    let steps = catalog_program();
    let mut held = 0;
    for kind in OperationKind::ALL {
        if !steps.iter().any(|step| {
            kind_of(&step.operation) == *kind && lower(&step.operation, &step.inputs, &[]).is_some()
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
        Scenario::KernelsReached => kernels_reached(&catalog_program()),
        Scenario::KernelNoLoweringRuns => {
            let steps: Vec<Step> = catalog_program()
                .into_iter()
                .filter(|step| {
                    !matches!(
                        step.operation,
                        Operation::GroupCount { .. } | Operation::GroupDigest { .. }
                    )
                })
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
