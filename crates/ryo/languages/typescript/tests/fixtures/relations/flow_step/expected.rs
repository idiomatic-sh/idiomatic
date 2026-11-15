// The rows `relations/flow_step/input.ts` must give.
// `h`: oxc's block after `return;` holds its unreachable mark at place 0 and `foo();` at place 1. `l`: `break outer;` is
// break-labeled.

use ryo::languages::contract::{Expected, FlowStepExpectedRow, expected_rows, file, name, node, number};

/// The rows, in the order the language gives them.
const ROWS: &[FlowStepExpectedRow] = &[
    FlowStepExpectedRow { block: number(3), ordinal: number(0), step_kind: name("statement"), node: node(14), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(4), ordinal: number(0), step_kind: name("condition"), node: node(18), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(5), ordinal: number(0), step_kind: name("statement"), node: node(22), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(6), ordinal: number(0), step_kind: name("statement"), node: node(26), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(7), ordinal: number(0), step_kind: name("condition"), node: node(30), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(8), ordinal: number(0), step_kind: name("statement"), node: node(34), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(8), ordinal: number(1), step_kind: name("statement"), node: node(37), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(9), ordinal: number(0), step_kind: name("statement"), node: node(49), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(9), ordinal: number(1), step_kind: name("statement"), node: node(52), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(10), ordinal: number(0), step_kind: name("statement"), node: node(62), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(13), ordinal: number(0), step_kind: name("return-undefined"), node: node(83), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(14), ordinal: number(1), step_kind: name("statement"), node: node(87), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(17), ordinal: number(0), step_kind: name("statement"), node: node(108), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(20), ordinal: number(0), step_kind: name("statement"), node: node(111), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(20), ordinal: number(1), step_kind: name("statement"), node: node(114), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(21), ordinal: number(0), step_kind: name("statement"), node: node(127), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(21), ordinal: number(1), step_kind: name("statement"), node: node(130), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(22), ordinal: number(0), step_kind: name("statement"), node: node(142), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(22), ordinal: number(1), step_kind: name("statement"), node: node(145), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(26), ordinal: number(0), step_kind: name("statement"), node: node(168), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(26), ordinal: number(1), step_kind: name("statement"), node: node(173), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(29), ordinal: number(0), step_kind: name("statement"), node: node(181), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(29), ordinal: number(1), step_kind: name("break-labeled"), node: node(184), file: file("flow_step/input.ts") },
    FlowStepExpectedRow { block: number(35), ordinal: number(0), step_kind: name("statement"), node: node(208), file: file("flow_step/input.ts") },
];

/// The rows the fixture's input must give, as the relations contract compares them.
pub(super) fn expected() -> Expected {
    expected_rows(ROWS)
}
