//! The services of the `walking` part of the `code` concept: one constructor and one reader for each of
//! the words that walk the tree and the edges a body yields: the one place each is written and read.

use crate::domain::code::types::{CodeWord, Op, PoolAt, Short, WordSlot};
use crate::domain::code::walking::types::{
    AxisWord, ChildrenOfFieldWord, ChildrenWord, ClosureWord, NearestWord, WithinWord,
};
use crate::domain::steps::types::Register;
use ryo_language::domain::node_table::types::FieldCode;

/// The `Children` word of `operands`.
#[must_use]
pub(crate) fn children(operands: ChildrenWord) -> CodeWord {
    CodeWord {
        op: Op::Children,
        a: WordSlot(operands.parent.0),
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Children` word.
#[must_use]
pub(crate) const fn children_of(word: CodeWord) -> ChildrenWord {
    ChildrenWord {
        parent: Register(word.a.0),
        out: Register(word.b.0),
    }
}

/// The `ChildrenOfField` word of `operands`.
#[must_use]
pub(crate) fn children_of_field(operands: ChildrenOfFieldWord) -> CodeWord {
    CodeWord {
        op: Op::ChildrenOfField,
        short: Short(operands.field.0),
        a: WordSlot(operands.parent.0),
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of a `ChildrenOfField` word.
#[must_use]
pub(crate) const fn children_of_field_of(word: CodeWord) -> ChildrenOfFieldWord {
    ChildrenOfFieldWord {
        field: FieldCode(word.short.0),
        parent: Register(word.a.0),
        out: Register(word.b.0),
    }
}

/// The `Axis` word of `operands`.
#[must_use]
pub(crate) fn axis(operands: AxisWord) -> CodeWord {
    CodeWord {
        op: Op::Axis,
        flag: operands.direction,
        a: WordSlot(operands.start.0),
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of an `Axis` word.
#[must_use]
pub(crate) const fn axis_of(word: CodeWord) -> AxisWord {
    AxisWord {
        direction: word.flag,
        start: Register(word.a.0),
        out: Register(word.b.0),
    }
}

/// The `Within` word of `operands`.
#[must_use]
pub(crate) fn within(operands: WithinWord) -> CodeWord {
    CodeWord {
        op: Op::Within,
        a: WordSlot(operands.ancestor.0),
        b: WordSlot(operands.descendant.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Within` word.
#[must_use]
pub(crate) const fn within_of(word: CodeWord) -> WithinWord {
    WithinWord {
        ancestor: Register(word.a.0),
        descendant: Register(word.b.0),
    }
}

/// The `Nearest` word of `operands`.
#[must_use]
pub(crate) fn nearest(operands: NearestWord) -> CodeWord {
    CodeWord {
        op: Op::Nearest,
        flag: operands.flag,
        a: WordSlot(operands.start.0),
        b: WordSlot(operands.out.0),
        x: WordSlot(operands.operands.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Nearest` word.
#[must_use]
pub(crate) const fn nearest_of(word: CodeWord) -> NearestWord {
    NearestWord {
        flag: word.flag,
        start: Register(word.a.0),
        out: Register(word.b.0),
        operands: PoolAt(word.x.0),
    }
}

/// The `NearestDerived` word of `operands`.
#[must_use]
pub(crate) fn nearest_derived(operands: NearestWord) -> CodeWord {
    CodeWord {
        op: Op::NearestDerived,
        flag: operands.flag,
        a: WordSlot(operands.start.0),
        b: WordSlot(operands.out.0),
        x: WordSlot(operands.operands.0),
        ..CodeWord::default()
    }
}

/// The operands of a `NearestDerived` word.
#[must_use]
pub(crate) const fn nearest_derived_of(word: CodeWord) -> NearestWord {
    NearestWord {
        flag: word.flag,
        start: Register(word.a.0),
        out: Register(word.b.0),
        operands: PoolAt(word.x.0),
    }
}

/// The `Closure` word of `operands`.
#[must_use]
pub(crate) fn closure(operands: ClosureWord) -> CodeWord {
    CodeWord {
        op: Op::Closure,
        a: WordSlot(operands.seed.0),
        b: WordSlot(operands.out.0),
        x: WordSlot(operands.operands.0),
        ..CodeWord::default()
    }
}

/// The operands of a `Closure` word.
#[must_use]
pub(crate) const fn closure_of(word: CodeWord) -> ClosureWord {
    ClosureWord {
        seed: Register(word.a.0),
        out: Register(word.b.0),
        operands: PoolAt(word.x.0),
    }
}
