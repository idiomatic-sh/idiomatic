//! The walk of a program's steps: each step once, in pre-order, through inline bodies and never into a definition. The counted
//! rows nest a body in a body to a depth and to twice that depth: the steps met grow with the depth, by the same amount a level.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::steps::traversal::service::each_step;
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, PlacedStep, Polarity, ProgramStep, Register, RegisterList, StepBody,
    StepMemo, StepOperand, StepOperands, StepProgram,
};
use std::borrow::Cow;

/// The program a row walks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// An `Exists` over an inline body and a `Yield`, nested this deep.
    Nested(u32),
    /// A `Let` into 1; an `Or` of two inline bodies, a `Let` into 2 and a `Let` into 3, and of a use of a definition; a `Let` into 4.
    Ordered,
}

/// What a walk met.
#[derive(Debug, PartialEq, Eq)]
struct Walked {
    /// How many times the visitor ran.
    steps: usize,
    /// The registers the steps met write, in the order met.
    writes: Cow<'static, [u32]>,
}

type Input = Scenario;
type Value = Walked;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_walk_meets_each_step_once_at_depth_ten"),
        word: Word::Succeeds,
        input: || Scenario::Nested(10),
        expect: Expect::Value(Walked {
            steps: 21,
            writes: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("a_walk_meets_each_step_once_at_depth_twenty"),
        word: Word::Succeeds,
        input: || Scenario::Nested(20),
        expect: Expect::Value(Walked {
            steps: 41,
            writes: Cow::Borrowed(&[]),
        }),
    },
    Case {
        name: CaseName("a_walk_is_in_pre_order_and_goes_into_no_definition"),
        word: Word::Succeeds,
        input: || Scenario::Ordered,
        expect: Expect::Value(Walked {
            steps: 5,
            writes: Cow::Borrowed(&[1, 2, 3, 4]),
        }),
    },
];

fn program<const N: usize>(steps: [ProgramStep; N]) -> StepProgram {
    StepProgram(Box::new(steps.map(|step| PlacedStep { point: None, step })))
}

fn exists(body: StepProgram) -> ProgramStep {
    ProgramStep::Exists {
        polarity: Polarity::Present,
        body: StepBody::Inline(body),
        memo: StepMemo::None,
    }
}

fn yielded() -> ProgramStep {
    ProgramStep::Yield {
        row: RegisterList::default(),
    }
}

fn nested(depth: u32) -> StepProgram {
    (0..depth).fold(program([yielded()]), |inner, _| {
        program([exists(inner), yielded()])
    })
}

fn set(out: u32) -> ProgramStep {
    ProgramStep::Let {
        from: StepOperand::File,
        out: Register(out),
    }
}

fn ordered() -> StepProgram {
    let used = StepBody::Definition {
        definition: DefinitionId(7),
        bound: ColumnMask::default(),
        collected: ColumnMask::default(),
        values: StepOperands::default(),
    };
    let either = ProgramStep::Or {
        bodies: Box::new([
            StepBody::Inline(program([set(2)])),
            StepBody::Inline(program([set(3)])),
            used,
        ]),
        outputs: RegisterList::default(),
        memo: StepMemo::None,
    };
    program([set(1), either, set(4)])
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let walked = match scenario {
        Scenario::Nested(depth) => nested(depth),
        Scenario::Ordered => ordered(),
    };
    let (mut steps, mut writes) = (0, Vec::new());
    each_step(&walked, &mut |_, touched| {
        steps += 1;
        writes.extend(touched.writes.iter().map(|register| register.0));
    });
    Ok(Walked {
        steps,
        writes: Cow::Owned(writes),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
