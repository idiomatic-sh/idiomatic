//! The services of the `anchoring` part of the `code` concept: one constructor and one reader for each of
//! the words that give the rows of a source: a kind's ids, the rows that hold values, the rows of a range, every row, and the rows of an ordered table nearest a bound: the one place each is written and read.

use crate::domain::code::anchoring::types::{
    EachKindWord, EachMatchMadeWord, EachMatchManyWord, EachMatchWord, EachRangeWord,
    EachRowFilledWord, EachRowMadeWord, EachRowNodeWord, LatestWord,
};
use crate::domain::code::types::{
    CodeWord, IndexSite, Op, PoolAt, RelationNumber, Short, WordSlot,
};
use crate::domain::steps::types::{MadeTableId, Register};
use ryo_relations::domain::relations::types::KindBits;

/// The `EachKind` word of `operands`.
#[must_use]
pub(crate) fn each_kind(operands: EachKindWord) -> CodeWord {
    CodeWord {
        op: Op::EachKind,
        flag: operands.flag,
        short: Short(operands.kind.0),
        a: WordSlot(operands.out.0),
        x: WordSlot(operands.kinds.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachKind` word.
#[must_use]
pub(crate) const fn each_kind_of(word: CodeWord) -> EachKindWord {
    EachKindWord {
        flag: word.flag,
        kind: KindBits(word.short.0),
        out: Register(word.a.0),
        kinds: PoolAt(word.x.0),
    }
}

/// The `EachMatchNode` word of `operands`.
#[must_use]
pub(crate) fn each_match_node(operands: EachMatchWord) -> CodeWord {
    CodeWord {
        op: Op::EachMatchNode,
        flag: operands.flag,
        a: WordSlot(operands.out.0),
        b: operands.value,
        x: WordSlot(operands.index.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachMatchNode` word.
#[must_use]
pub(crate) const fn each_match_node_of(word: CodeWord) -> EachMatchWord {
    EachMatchWord {
        flag: word.flag,
        out: Register(word.a.0),
        value: word.b,
        index: IndexSite(word.x.0),
    }
}

/// The `EachMatchFilled` word of `operands`.
#[must_use]
pub(crate) fn each_match_filled(operands: EachMatchWord) -> CodeWord {
    CodeWord {
        op: Op::EachMatchFilled,
        flag: operands.flag,
        a: WordSlot(operands.out.0),
        b: operands.value,
        x: WordSlot(operands.index.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachMatchFilled` word.
#[must_use]
pub(crate) const fn each_match_filled_of(word: CodeWord) -> EachMatchWord {
    EachMatchWord {
        flag: word.flag,
        out: Register(word.a.0),
        value: word.b,
        index: IndexSite(word.x.0),
    }
}

/// The `EachMatchFilledMany` word of `operands`.
#[must_use]
pub(crate) fn each_match_filled_many(operands: EachMatchManyWord) -> CodeWord {
    CodeWord {
        op: Op::EachMatchFilledMany,
        short: operands.values,
        a: WordSlot(operands.out.0),
        b: WordSlot(operands.operands.0),
        x: WordSlot(operands.index.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachMatchFilledMany` word.
#[must_use]
pub(crate) const fn each_match_filled_many_of(word: CodeWord) -> EachMatchManyWord {
    EachMatchManyWord {
        values: word.short,
        out: Register(word.a.0),
        operands: PoolAt(word.b.0),
        index: IndexSite(word.x.0),
    }
}

/// The `EachMatchMade` word of `operands`.
#[must_use]
pub(crate) fn each_match_made(operands: EachMatchMadeWord) -> CodeWord {
    CodeWord {
        op: Op::EachMatchMade,
        flag: operands.flag,
        short: operands.values,
        a: WordSlot(operands.out.0),
        b: operands.value,
        x: WordSlot(operands.index.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachMatchMade` word.
#[must_use]
pub(crate) const fn each_match_made_of(word: CodeWord) -> EachMatchMadeWord {
    EachMatchMadeWord {
        flag: word.flag,
        values: word.short,
        out: Register(word.a.0),
        value: word.b,
        index: IndexSite(word.x.0),
    }
}

/// The `EachRange` word of `operands`.
#[must_use]
pub(crate) fn each_range(operands: EachRangeWord) -> CodeWord {
    CodeWord {
        op: Op::EachRange,
        flag: operands.flag,
        short: operands.values,
        a: WordSlot(operands.out.0),
        b: WordSlot(operands.operands.0),
        x: WordSlot(operands.index.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachRange` word.
#[must_use]
pub(crate) const fn each_range_of(word: CodeWord) -> EachRangeWord {
    EachRangeWord {
        flag: word.flag,
        values: word.short,
        out: Register(word.a.0),
        operands: PoolAt(word.b.0),
        index: IndexSite(word.x.0),
    }
}

/// The `EachRowNode` word of `operands`.
#[must_use]
pub(crate) fn each_row_node(operands: EachRowNodeWord) -> CodeWord {
    CodeWord {
        op: Op::EachRowNode,
        flag: operands.flag,
        a: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachRowNode` word.
#[must_use]
pub(crate) const fn each_row_node_of(word: CodeWord) -> EachRowNodeWord {
    EachRowNodeWord {
        flag: word.flag,
        out: Register(word.a.0),
    }
}

/// The `EachRowFilled` word of `operands`.
#[must_use]
pub(crate) fn each_row_filled(operands: EachRowFilledWord) -> CodeWord {
    CodeWord {
        op: Op::EachRowFilled,
        a: WordSlot(operands.out.0),
        x: WordSlot(operands.relation.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachRowFilled` word.
#[must_use]
pub(crate) const fn each_row_filled_of(word: CodeWord) -> EachRowFilledWord {
    EachRowFilledWord {
        out: Register(word.a.0),
        relation: RelationNumber(word.x.0),
    }
}

/// The `EachRowMade` word of `operands`.
#[must_use]
pub(crate) fn each_row_made(operands: EachRowMadeWord) -> CodeWord {
    CodeWord {
        op: Op::EachRowMade,
        flag: operands.flag,
        a: WordSlot(operands.out.0),
        x: WordSlot(operands.table.0),
        ..CodeWord::default()
    }
}

/// The operands of an `EachRowMade` word.
#[must_use]
pub(crate) const fn each_row_made_of(word: CodeWord) -> EachRowMadeWord {
    EachRowMadeWord {
        flag: word.flag,
        out: Register(word.a.0),
        table: MadeTableId(word.x.0),
    }
}

/// The `Latest` word of `operands`.
#[must_use]
pub(crate) fn latest(operands: LatestWord) -> CodeWord {
    CodeWord {
        op: Op::Latest,
        flag: operands.flag,
        short: operands.keys,
        a: WordSlot(operands.out.0),
        b: operands.bound,
        x: WordSlot(operands.operands.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Latest` word.
#[must_use]
pub(crate) const fn latest_of(word: CodeWord) -> LatestWord {
    LatestWord {
        flag: word.flag,
        keys: word.short,
        out: Register(word.a.0),
        bound: word.b,
        operands: PoolAt(word.x.0),
    }
}
