//! The `EachMatch` words' cases: the key is a cell for each value, from a register or a constant, one or several; the rows
//! the loop was handed are bound in order as their handles, a node's with its file.

use super::item::EachMatch;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::anchoring::service::{
    each_match_filled_many, each_match_made, each_match_node,
};
use crate::domain::code::anchoring::types::{EachMatchMadeWord, EachMatchManyWord, EachMatchWord};
use crate::domain::code::constants::CONSTANT;
use crate::domain::code::records::service::{next_place, push_operand};
use crate::domain::code::records::types::PooledOperand;
use crate::domain::code::types::{
    CodeEntry, CodeWord, ConstantNumber, IndexSite, ProgramCode, Short, WordFlag, WordSlot,
};
use crate::domain::indexes::types::{RowRun, SourceRow};
use crate::domain::made_tables::types::KeyFrom;
use crate::domain::programs::collections::instructions::interface::{Flow, InstructionsItem};
use crate::domain::programs::fixtures::{Idle, framed, held};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::SeamRows;
use crate::domain::steps::types::Register;
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::Count;
use std::borrow::Cow;

/// The word a case runs. Register 1 holds 30; constant 0 is 60; the fill side's rows hold 4, 9 and 2, and the loop was
/// handed the last two.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A made table by one register.
    MadeRegister,
    /// A made table by one constant.
    MadeConstant,
    /// A filled relation by a register and a constant.
    FilledMany,
    /// The node table by one register.
    Node,
}

type Input = Asked;
/// The key's cells; 99; 1 when a cell of the key came from a register; 99; each handle bound; 99; the key room's length
/// after the item's cells were pushed.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_key_of_one_register"),
        word: Word::Succeeds,
        input: || Asked::MadeRegister,
        expect: Expect::Value(Cow::Borrowed(&[30, 99, 1, 99, 9, 2, 99, 2])),
    },
    Case {
        name: CaseName("a_key_of_one_constant"),
        word: Word::Succeeds,
        input: || Asked::MadeConstant,
        expect: Expect::Value(Cow::Borrowed(&[60, 99, 0, 99, 9, 2, 99, 2])),
    },
    Case {
        name: CaseName("a_key_of_several_values_in_the_columns_order"),
        word: Word::Succeeds,
        input: || Asked::FilledMany,
        expect: Expect::Value(Cow::Borrowed(&[30, 60, 99, 1, 99, 9, 2, 99, 3])),
    },
    Case {
        name: CaseName("a_nodes_handle_is_its_id_with_the_file"),
        word: Word::Succeeds,
        input: || Asked::Node,
        expect: Expect::Value(Cow::Borrowed(&[
            30,
            99,
            1,
            99,
            30_064_771_081,
            30_064_771_074,
            99,
            2,
        ])),
    },
];

fn subject(asked: Input) -> Result<Value, Failure> {
    let mut code = ProgramCode {
        constants: vec![CellWord(60)],
        pool: vec![WordSlot(0)],
        ..ProgramCode::default()
    };
    let operands = next_place(&code.pool);
    push_operand(&mut code.pool, PooledOperand::Register(Register(1)));
    push_operand(&mut code.pool, PooledOperand::Constant(ConstantNumber(0)));
    let (out, index) = (Register(0), IndexSite(0));
    let word: CodeWord = match asked {
        Asked::MadeRegister => each_match_made(EachMatchMadeWord {
            flag: WordFlag(0),
            values: Short(0),
            out,
            value: WordSlot(1),
            index,
        }),
        Asked::MadeConstant => each_match_made(EachMatchMadeWord {
            flag: CONSTANT,
            values: Short(0),
            out,
            value: WordSlot(0),
            index,
        }),
        Asked::FilledMany => each_match_filled_many(EachMatchManyWord {
            values: Short(2),
            out,
            operands,
            index,
        }),
        Asked::Node => each_match_node(EachMatchWord {
            flag: WordFlag(0),
            out,
            value: WordSlot(1),
            index,
        }),
    };
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(2);
    held.state.registers[1] = CellWord(30);
    held.state.key.push(CellWord(1));
    held.store
        .indexes
        .rows
        .extend([SourceRow(4), SourceRow(9), SourceRow(2)]);
    let (all, _found) = framed(&programs, &mut held, &Idle, |frame| {
        let mut all = Vec::new();
        let Some(key) = EachMatch::key(frame, word) else {
            return all;
        };
        all.extend(frame.state.key[key.start.0..].iter().map(|cell| cell.0));
        all.extend([99, u64::from(matches!(key.from, KeyFrom::Row)), 99]);
        frame.state.rows = SeamRows {
            run: RowRun {
                start: Count(1),
                end: Count(3),
            },
            tag: CellWord(0),
        };
        if EachMatch::run(frame, word, CodeEntry(4)) != Flow::Back {
            return all;
        }
        let Some(mut cursor) = frame.state.frames.last().copied() else {
            return all;
        };
        while EachMatch::next(frame, word, &mut cursor).is_some() {
            all.push(frame.state.registers[0].0);
        }
        all.extend([99, u64::try_from(frame.state.key.len()).unwrap_or(u64::MAX)]);
        all
    });
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
