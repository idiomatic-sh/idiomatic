//! A node the chain does not stream is held at seam C: a capture read by two filters holds its rows until the second has
//! read it, and an emitted capture holds them for its engine use after its one reader ran; each answer as its held twin's.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{DEFINES, defines_rows};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::logical_program::value::LogicalProgram;
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{
    LogicalNode, NodeBindings, NodeRows, PlanInputList, PlanNodeId,
};
use crate::inbound::use_cases::run::fixtures::streams::{
    against, select_of, shown, span, split_names,
};
use crate::inbound::use_cases::run::fixtures::{TableInputs, TextResources, planned, ran};
use ryo_values::domain::plan_parts::types::{StepScope, Test};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    TwoReaders,
    EmittedMiddle,
}
/// Whether the capture holds rows after each emit ran, in order, then whether every emit answered as in the twin.
type Value = Cow<'static, str>;
type Input = Scenario;
type Failure = PortableOperatorFault;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_node_with_two_readers_is_held"),
        word: Word::Succeeds,
        input: || Scenario::TwoReaders,
        expect: Expect::Value(Cow::Borrowed("held absent same")),
    },
    Case {
        name: CaseName("an_emitted_middle_node_is_held"),
        word: Word::Succeeds,
        input: || Scenario::EmittedMiddle,
        expect: Expect::Value(Cow::Borrowed("held same")),
    },
];

/// Runs node 0, the capture, and filters 1 and 2 of it, the filters (and the capture too when `capture_emitted`) emitted,
/// each emit in node order; with `twin`, the capture is held by the engine as well. Answers whether the capture holds rows
/// after each filter's emit, and each filter's rows as text.
fn runs(capture_emitted: bool, twin: bool) -> Result<(Vec<&'static str>, Vec<String>), Failure> {
    let texts = ["abc", "ab", "xab", "abab", "a"];
    let mut resources = TextResources::new(&texts);
    let named: Vec<_> = ["abc", "ab", "xab", "abab"]
        .into_iter()
        .map(|name| (resources.symbol(name), span(0, 0)))
        .collect();
    let table = defines_rows(&named);
    let inputs = TableInputs::new(&[(DEFINES, &table)]);
    let steps = [
        split_names(),
        select_of(0, vec![against(Test::Equal, 7, "a")]),
        select_of(0, vec![against(Test::Equal, 7, "ab")]),
    ];
    let mut program = LogicalProgram::new();
    for step in &steps {
        let _id = program.node(LogicalNode {
            operation: &step.operation,
            inputs: PlanInputList(&step.inputs),
            bindings: NodeBindings(&[]),
            scope: StepScope::PerFile,
        });
    }
    let emits: Vec<PlanNodeId> = if capture_emitted {
        vec![PlanNodeId(0), PlanNodeId(1)]
    } else {
        vec![PlanNodeId(1), PlanNodeId(2)]
    };
    for (rule, node) in emits.iter().enumerate() {
        program.emit(RuleOrdinal(u32::try_from(rule).unwrap_or(u32::MAX)), *node);
    }
    if twin {
        program.hold(PlanNodeId(0));
    }
    let plan = planned(&program)?;
    let mut buffers = FileBuffers::default();
    let mut held = Vec::new();
    let mut answers = Vec::new();
    for node in emits.iter().filter(|node| **node != PlanNodeId(0)) {
        ran(&plan, *node, &mut buffers, &inputs, &mut resources)?;
        held.push(match buffers.rows(PlanNodeId(0)) {
            NodeRows::Materialized(_) => "held",
            NodeRows::Absent => "absent",
        });
        if let NodeRows::Materialized(rows) = buffers.rows(*node) {
            answers.push(
                rows.cells
                    .iter()
                    .map(|cell| shown(*cell, &resources))
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
    }
    Ok((held, answers))
}

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    let emitted = matches!(scenario, Scenario::EmittedMiddle);
    let (held, answers) = runs(emitted, false)?;
    let (_, twin) = runs(emitted, true)?;
    Ok(Cow::Owned(format!(
        "{} {}",
        held.join(" "),
        if answers == twin { "same" } else { "different" }
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
