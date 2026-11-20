//! The services of the `deciding` part of the `code` concept: one constructor and one reader for each of
//! the words that read a cell, test, call a meaning and write a register: the one place each is written and read.

use crate::domain::code::deciding::types::{
    CallWord, ConstantTestWord, LetWord, ReadFilledWord, ReadMadeWord, ReadNodeWord,
    RegisterTestWord, RenderWord, SetTestWord, TestWord,
};
use crate::domain::code::types::{
    CodeWord, ConstantNumber, Op, PoolAt, RelationNumber, Short, WordSlot,
};
use crate::domain::steps::types::{MadeColumn, MadeTableId, RegexId, Register};

/// The `ReadNode` word of `operands`.
#[must_use]
pub(crate) fn read_node(operands: ReadNodeWord) -> CodeWord {
    CodeWord {
        op: Op::ReadNode,
        short: operands.column,
        a: WordSlot(operands.row.0),
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of a `ReadNode` word.
#[must_use]
pub(crate) const fn read_node_of(word: CodeWord) -> ReadNodeWord {
    ReadNodeWord {
        column: word.short,
        row: Register(word.a.0),
        out: Register(word.b.0),
    }
}

/// The `ReadFilled` word of `operands`.
#[must_use]
pub(crate) fn read_filled(operands: ReadFilledWord) -> CodeWord {
    CodeWord {
        op: Op::ReadFilled,
        short: operands.place,
        a: WordSlot(operands.row.0),
        b: WordSlot(operands.out.0),
        x: WordSlot(operands.relation.0),
        ..CodeWord::default()
    }
}

/// The operands of a `ReadFilled` word.
#[must_use]
pub(crate) const fn read_filled_of(word: CodeWord) -> ReadFilledWord {
    ReadFilledWord {
        place: word.short,
        row: Register(word.a.0),
        out: Register(word.b.0),
        relation: RelationNumber(word.x.0),
    }
}

/// The `ReadMade` word of `operands`.
#[must_use]
pub(crate) fn read_made(operands: ReadMadeWord) -> CodeWord {
    CodeWord {
        op: Op::ReadMade,
        short: Short(operands.column.0),
        a: WordSlot(operands.row.0),
        b: WordSlot(operands.out.0),
        x: WordSlot(operands.table.0),
        ..CodeWord::default()
    }
}

/// The operands of a `ReadMade` word.
#[must_use]
pub(crate) const fn read_made_of(word: CodeWord) -> ReadMadeWord {
    ReadMadeWord {
        column: MadeColumn(word.short.0),
        row: Register(word.a.0),
        out: Register(word.b.0),
        table: MadeTableId(word.x.0),
    }
}

/// The `Test` word of `operands`.
#[must_use]
pub(crate) fn test(operands: TestWord) -> CodeWord {
    CodeWord {
        op: Op::Test,
        flag: operands.flag,
        short: operands.comparison,
        a: operands.left,
        b: operands.right,
        x: WordSlot(operands.regex.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Test` word.
#[must_use]
pub(crate) const fn test_of(word: CodeWord) -> TestWord {
    TestWord {
        flag: word.flag,
        comparison: word.short,
        left: word.a,
        right: word.b,
        regex: RegexId(word.x.0),
    }
}

/// The `TestEqualConstant` word of `operands`.
#[must_use]
pub(crate) fn test_equal_constant(operands: ConstantTestWord) -> CodeWord {
    CodeWord {
        op: Op::TestEqualConstant,
        a: WordSlot(operands.register.0),
        b: WordSlot(operands.constant.0),
        ..CodeWord::default()
    }
}

/// The operands of a `TestEqualConstant` word.
#[must_use]
pub(crate) const fn test_equal_constant_of(word: CodeWord) -> ConstantTestWord {
    ConstantTestWord {
        register: Register(word.a.0),
        constant: ConstantNumber(word.b.0),
    }
}

/// The `TestNotEqualConstant` word of `operands`.
#[must_use]
pub(crate) fn test_not_equal_constant(operands: ConstantTestWord) -> CodeWord {
    CodeWord {
        op: Op::TestNotEqualConstant,
        a: WordSlot(operands.register.0),
        b: WordSlot(operands.constant.0),
        ..CodeWord::default()
    }
}

/// The operands of a `TestNotEqualConstant` word.
#[must_use]
pub(crate) const fn test_not_equal_constant_of(word: CodeWord) -> ConstantTestWord {
    ConstantTestWord {
        register: Register(word.a.0),
        constant: ConstantNumber(word.b.0),
    }
}

/// The `TestEqualRegister` word of `operands`.
#[must_use]
pub(crate) fn test_equal_register(operands: RegisterTestWord) -> CodeWord {
    CodeWord {
        op: Op::TestEqualRegister,
        a: WordSlot(operands.left.0),
        b: WordSlot(operands.right.0),
        ..CodeWord::default()
    }
}

/// The operands of a `TestEqualRegister` word.
#[must_use]
pub(crate) const fn test_equal_register_of(word: CodeWord) -> RegisterTestWord {
    RegisterTestWord {
        left: Register(word.a.0),
        right: Register(word.b.0),
    }
}

/// The `TestNotEqualRegister` word of `operands`.
#[must_use]
pub(crate) fn test_not_equal_register(operands: RegisterTestWord) -> CodeWord {
    CodeWord {
        op: Op::TestNotEqualRegister,
        a: WordSlot(operands.left.0),
        b: WordSlot(operands.right.0),
        ..CodeWord::default()
    }
}

/// The operands of a `TestNotEqualRegister` word.
#[must_use]
pub(crate) const fn test_not_equal_register_of(word: CodeWord) -> RegisterTestWord {
    RegisterTestWord {
        left: Register(word.a.0),
        right: Register(word.b.0),
    }
}

/// The `TestInConstants` word of `operands`.
#[must_use]
pub(crate) fn test_in_constants(operands: SetTestWord) -> CodeWord {
    CodeWord {
        op: Op::TestInConstants,
        flag: operands.flag,
        a: WordSlot(operands.register.0),
        b: WordSlot(operands.constants.0),
        ..CodeWord::default()
    }
}

/// The operands of a `TestInConstants` word.
#[must_use]
pub(crate) const fn test_in_constants_of(word: CodeWord) -> SetTestWord {
    SetTestWord {
        flag: word.flag,
        register: Register(word.a.0),
        constants: PoolAt(word.b.0),
    }
}

/// The `Call` word of `operands`.
#[must_use]
pub(crate) fn call(operands: CallWord) -> CodeWord {
    CodeWord {
        op: Op::Call,
        short: operands.meaning,
        a: WordSlot(operands.arguments.0),
        b: WordSlot(operands.outputs.0),
        x: WordSlot(operands.regex.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Call` word.
#[must_use]
pub(crate) const fn call_of(word: CodeWord) -> CallWord {
    CallWord {
        meaning: word.short,
        arguments: PoolAt(word.a.0),
        outputs: PoolAt(word.b.0),
        regex: RegexId(word.x.0),
    }
}

/// The `Render` word of `operands`.
#[must_use]
pub(crate) fn render(operands: RenderWord) -> CodeWord {
    CodeWord {
        op: Op::Render,
        short: operands.context,
        a: WordSlot(operands.node.0),
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Render` word.
#[must_use]
pub(crate) const fn render_of(word: CodeWord) -> RenderWord {
    RenderWord {
        context: word.short,
        node: Register(word.a.0),
        out: Register(word.b.0),
    }
}

/// The `Let` word of `operands`.
#[must_use]
pub(crate) fn let_value(operands: LetWord) -> CodeWord {
    CodeWord {
        op: Op::Let,
        flag: operands.flag,
        a: operands.from,
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Let` word.
#[must_use]
pub(crate) const fn let_value_of(word: CodeWord) -> LetWord {
    LetWord {
        flag: word.flag,
        from: word.a,
        out: Register(word.b.0),
    }
}
