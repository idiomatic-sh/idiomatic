//! The services of the `controlling` part of the `code` concept: one constructor and one reader for each of
//! the words over bodies and the words that end a program or hand a row over: the one place each is written and read.

use crate::domain::code::controlling::types::{EmitWord, ExistsWord, ForkWord, OrWord, YieldWord};
use crate::domain::code::types::{CodeWord, MemoSite, Op, PoolAt, WordSlot};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The `End` word: it has no operand, so it has no reader.
#[must_use]
pub(crate) fn end() -> CodeWord {
    CodeWord {
        op: Op::End,
        ..CodeWord::default()
    }
}

/// The `Exists` word of `operands`.
#[must_use]
pub(crate) fn exists(operands: ExistsWord) -> CodeWord {
    CodeWord {
        op: Op::Exists,
        flag: operands.flag,
        short: operands.keys,
        a: WordSlot(operands.body.0),
        b: WordSlot(operands.key.0),
        x: WordSlot(operands.memo.0),
        ..CodeWord::default()
    }
}

/// The operands of an `Exists` word.
#[must_use]
pub(crate) const fn exists_of(word: CodeWord) -> ExistsWord {
    ExistsWord {
        flag: word.flag,
        keys: word.short,
        body: PoolAt(word.a.0),
        key: PoolAt(word.b.0),
        memo: MemoSite(word.x.0),
    }
}

/// The `Or` word of `operands`.
#[must_use]
pub(crate) fn or(operands: OrWord) -> CodeWord {
    CodeWord {
        op: Op::Or,
        flag: operands.flag,
        short: operands.bodies,
        a: WordSlot(operands.records.0),
        b: WordSlot(operands.outputs.0),
        x: WordSlot(operands.memo.0),
        ..CodeWord::default()
    }
}

/// The operands of an `Or` word.
#[must_use]
pub(crate) const fn or_of(word: CodeWord) -> OrWord {
    OrWord {
        flag: word.flag,
        bodies: word.short,
        records: PoolAt(word.a.0),
        outputs: PoolAt(word.b.0),
        memo: MemoSite(word.x.0),
    }
}

/// The `Fork` word of `operands`.
#[must_use]
pub(crate) fn fork(operands: ForkWord) -> CodeWord {
    CodeWord {
        op: Op::Fork,
        short: operands.slots,
        a: WordSlot(operands.entries.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Fork` word.
#[must_use]
pub(crate) const fn fork_of(word: CodeWord) -> ForkWord {
    ForkWord {
        slots: word.short,
        entries: PoolAt(word.a.0),
    }
}

/// The `Yield` word of `operands`.
#[must_use]
pub(crate) fn yield_row(operands: YieldWord) -> CodeWord {
    CodeWord {
        op: Op::Yield,
        short: operands.cells,
        a: operands.row,
        ..CodeWord::default()
    }
}

/// The operands of a `Yield` word.
#[must_use]
pub(crate) const fn yield_row_of(word: CodeWord) -> YieldWord {
    YieldWord {
        cells: word.short,
        row: word.a,
    }
}

/// The `Emit` word of `operands`.
#[must_use]
pub(crate) fn emit(operands: EmitWord) -> CodeWord {
    CodeWord {
        op: Op::Emit,
        flag: operands.flag,
        short: operands.columns,
        a: WordSlot(operands.rule.0),
        b: operands.input,
        x: WordSlot(operands.operands.0),
        ..CodeWord::default()
    }
}

/// The operands of an `Emit` word.
#[must_use]
pub(crate) const fn emit_of(word: CodeWord) -> EmitWord {
    EmitWord {
        flag: word.flag,
        columns: word.short,
        rule: SelectedRuleOrdinal(word.a.0),
        input: word.b,
        operands: PoolAt(word.x.0),
    }
}
