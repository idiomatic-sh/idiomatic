//! Type-context classification reads a bounded part of the tree per host, never a whole list or
//! a whole ancestor chain.
use super::super::service::{ancestors, operand_place};
use super::super::types::{TypeContext, TypingForm};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    nodes::types::{Category, Node},
    syntax::{
        service::{kind, literal},
        types::{Kind, Literal},
    },
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Position, Tree};

/// A bounded-cost scenario.
#[derive(Clone, Copy, Debug)]
enum Input {
    /// Classify the second and the last of this many operands of each typing form.
    Operands(u32),
    /// Walk the ancestors of a call's quoted operand nested this many blocks deep.
    Blocks(usize),
}
/// Whether the reads stayed within the scenario's bound with the expected answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cost {
    /// Within the bound.
    Bounded,
    /// Over the bound, or an answer differed.
    Exceeded,
}
/// The source could not be parsed.
type Failure = ();

/// The replaced code scanned the operand list twice per argument, reading all 10,000 operands
/// for the last one, and walked every ancestor to the module, over 64 blocks for a nested host.
const CASES: &[Case<Input, Cost, Failure>] = &[
    Case {
        name: CaseName("a typing operand is placed reading at most two operands of 10,000"),
        word: Word::Succeeds,
        input: || Input::Operands(10_000),
        expect: Expect::Value(Cost::Bounded),
    },
    Case {
        name: CaseName("a quoted operand 64 blocks deep walks its expression, not its blocks"),
        word: Word::Succeeds,
        input: || Input::Blocks(64),
        expect: Expect::Value(Cost::Bounded),
    },
];

/// Count operand reads, or ancestor steps, for one scenario.
fn subject(input: Input) -> Result<Cost, Failure> {
    let bounded = match input {
        Input::Operands(count) => operands(count),
        Input::Blocks(depth) => blocks(depth)?,
    };
    Ok(if bounded {
        Cost::Bounded
    } else {
        Cost::Exceeded
    })
}

/// One operand to place: its form, its place among the operands, and the expected answer.
struct Placement {
    /// The resolved callable.
    form: TypingForm,
    /// The operand's place.
    argument: u32,
    /// Its type context.
    expected: TypeContext,
}

/// Every form answers as before for the second and the last operand, each within two reads.
fn operands(count: u32) -> bool {
    let last = count - 1;
    [
        Placement {
            form: TypingForm::Cast,
            argument: 1,
            expected: TypeContext::Value,
        },
        Placement {
            form: TypingForm::NewType,
            argument: 1,
            expected: TypeContext::Type,
        },
        Placement {
            form: TypingForm::TypeVar,
            argument: 1,
            expected: TypeContext::Type,
        },
        Placement {
            form: TypingForm::Cast,
            argument: last,
            expected: TypeContext::Value,
        },
        Placement {
            form: TypingForm::NewType,
            argument: last,
            expected: TypeContext::Value,
        },
        Placement {
            form: TypingForm::TypeVar,
            argument: last,
            expected: TypeContext::Type,
        },
    ]
    .into_iter()
    .all(|placement| {
        let mut reads = 0_u32;
        let answer = operand_place(
            (0..count).map(Position).inspect(|_| reads += 1),
            Position(placement.argument),
            placement.form,
            |_| Category::Expression,
        );
        answer == placement.expected && reads <= 2
    })
}

/// The quoted `cast` operand walks to its assignment statement and stops, whatever the depth.
fn blocks(depth: usize) -> Result<bool, Failure> {
    let mut source = String::from("from typing import cast\n");
    for level in 0..depth {
        source.push_str(&"    ".repeat(level));
        source.push_str("if a:\n");
    }
    source.push_str(&"    ".repeat(depth));
    source.push_str("x = cast(\"T\", v)\n");
    let parsed = parsed(source.as_bytes()).map_err(|_error| ())?;
    let tree = Tree {
        elements: &parsed.elements,
        source: ByteSlice(parsed.bytes),
        file: languages::FileId(0),
    };
    let host = languages::positions(tree.elements)
        .find(|&at| {
            kind(&tree, at) == Kind::Node(Node::Constant) && literal(&tree, at) == Literal::String
        })
        .ok_or(())?;
    let walked = ancestors(&tree, host).collect::<Vec<_>>();
    // The arguments, the call, and the assignment that ends the walk.
    Ok(walked.len() <= 4
        && walked
            .last()
            .is_some_and(|&last| kind(&tree, last) == Kind::Node(Node::Assign)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
