//! The services of the `filling` part of the `code` concept: one constructor and one reader for each of
//! the words of a pipeline's stages: the one place each is written and read.

use crate::domain::code::anchoring::types::LatestWord;
use crate::domain::code::deciding::types::{CallWord, RenderWord, TestWord};
use crate::domain::code::filling::types::{
    ProbeWord, ProductWord, ScanWord, StageReadWord, UseWord, WalkWord, WriteWord,
};
use crate::domain::code::types::{CodeWord, IndexSite, MemoSite, Op, PoolAt, Short, WordSlot};
use crate::domain::steps::types::{MadeTableId, RegexId, Register};
use ryo_language::domain::node_table::types::FieldCode;

/// The `StageScan` word of `operands`.
#[must_use]
pub(crate) fn stage_scan(operands: ScanWord) -> CodeWord {
    CodeWord {
        op: Op::StageScan,
        flag: operands.flag,
        a: WordSlot(operands.out.0),
        b: WordSlot(operands.constants.0),
        x: operands.read,
        ..CodeWord::default()
    }
}

/// The operands of a `StageScan` word.
#[must_use]
pub(crate) const fn stage_scan_of(word: CodeWord) -> ScanWord {
    ScanWord {
        flag: word.flag,
        out: Register(word.a.0),
        constants: PoolAt(word.b.0),
        read: word.x,
    }
}

/// The `StageProbe` word of `operands`.
#[must_use]
pub(crate) fn stage_probe(operands: ProbeWord) -> CodeWord {
    CodeWord {
        op: Op::StageProbe,
        flag: operands.flag,
        short: operands.values,
        a: WordSlot(operands.out.0),
        b: operands.probed,
        x: WordSlot(operands.index.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageProbe` word.
#[must_use]
pub(crate) const fn stage_probe_of(word: CodeWord) -> ProbeWord {
    ProbeWord {
        flag: word.flag,
        values: word.short,
        out: Register(word.a.0),
        probed: word.b,
        index: IndexSite(word.x.0),
    }
}

/// The `StageProduct` word of `operands`.
#[must_use]
pub(crate) fn stage_product(operands: ProductWord) -> CodeWord {
    CodeWord {
        op: Op::StageProduct,
        flag: operands.flag,
        a: WordSlot(operands.out.0),
        x: operands.read,
        ..CodeWord::default()
    }
}

/// The operands of a `StageProduct` word.
#[must_use]
pub(crate) const fn stage_product_of(word: CodeWord) -> ProductWord {
    ProductWord {
        flag: word.flag,
        out: Register(word.a.0),
        read: word.x,
    }
}

/// The `StageUse` word of `operands`.
#[must_use]
pub(crate) fn stage_use(operands: UseWord) -> CodeWord {
    CodeWord {
        op: Op::StageUse,
        flag: operands.flag,
        short: operands.outputs,
        a: WordSlot(operands.body.0),
        b: WordSlot(operands.registers.0),
        x: WordSlot(operands.memo.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageUse` word.
#[must_use]
pub(crate) const fn stage_use_of(word: CodeWord) -> UseWord {
    UseWord {
        flag: word.flag,
        outputs: word.short,
        body: PoolAt(word.a.0),
        registers: PoolAt(word.b.0),
        memo: MemoSite(word.x.0),
    }
}

/// The `StageLatest` word of `operands`.
#[must_use]
pub(crate) fn stage_latest(operands: LatestWord) -> CodeWord {
    CodeWord {
        op: Op::StageLatest,
        flag: operands.flag,
        short: operands.keys,
        a: WordSlot(operands.out.0),
        b: operands.bound,
        x: WordSlot(operands.operands.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageLatest` word.
#[must_use]
pub(crate) const fn stage_latest_of(word: CodeWord) -> LatestWord {
    LatestWord {
        flag: word.flag,
        keys: word.short,
        out: Register(word.a.0),
        bound: word.b,
        operands: PoolAt(word.x.0),
    }
}

/// The `StageRead` word of `operands`.
#[must_use]
pub(crate) fn stage_read(operands: StageReadWord) -> CodeWord {
    CodeWord {
        op: Op::StageRead,
        flag: operands.flag,
        short: operands.column,
        a: WordSlot(operands.row.0),
        b: WordSlot(operands.out.0),
        x: operands.read,
        ..CodeWord::default()
    }
}

/// The operands of a `StageRead` word.
#[must_use]
pub(crate) const fn stage_read_of(word: CodeWord) -> StageReadWord {
    StageReadWord {
        flag: word.flag,
        column: word.short,
        row: Register(word.a.0),
        out: Register(word.b.0),
        read: word.x,
    }
}

/// The `StageKeep` word of `operands`.
#[must_use]
pub(crate) fn stage_keep(operands: TestWord) -> CodeWord {
    CodeWord {
        op: Op::StageKeep,
        flag: operands.flag,
        short: operands.comparison,
        a: operands.left,
        b: operands.right,
        x: WordSlot(operands.regex.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageKeep` word.
#[must_use]
pub(crate) const fn stage_keep_of(word: CodeWord) -> TestWord {
    TestWord {
        flag: word.flag,
        comparison: word.short,
        left: word.a,
        right: word.b,
        regex: RegexId(word.x.0),
    }
}

/// The `StageCall` word of `operands`.
#[must_use]
pub(crate) fn stage_call(operands: CallWord) -> CodeWord {
    CodeWord {
        op: Op::StageCall,
        flag: operands.flag,
        short: operands.meaning,
        a: WordSlot(operands.arguments.0),
        b: WordSlot(operands.outputs.0),
        x: WordSlot(operands.regex.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageCall` word.
#[must_use]
pub(crate) const fn stage_call_of(word: CodeWord) -> CallWord {
    CallWord {
        flag: word.flag,
        meaning: word.short,
        arguments: PoolAt(word.a.0),
        outputs: PoolAt(word.b.0),
        regex: RegexId(word.x.0),
    }
}

/// The `StageRender` word of `operands`.
#[must_use]
pub(crate) fn stage_render(operands: RenderWord) -> CodeWord {
    CodeWord {
        op: Op::StageRender,
        short: operands.context,
        a: WordSlot(operands.node.0),
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageRender` word.
#[must_use]
pub(crate) const fn stage_render_of(word: CodeWord) -> RenderWord {
    RenderWord {
        context: word.short,
        node: Register(word.a.0),
        out: Register(word.b.0),
    }
}

/// The `StageWalk` word of `operands`.
#[must_use]
pub(crate) fn stage_walk(operands: WalkWord) -> CodeWord {
    CodeWord {
        op: Op::StageWalk,
        flag: operands.flag,
        short: Short(operands.field.0),
        a: WordSlot(operands.start.0),
        b: WordSlot(operands.out.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageWalk` word.
#[must_use]
pub(crate) const fn stage_walk_of(word: CodeWord) -> WalkWord {
    WalkWord {
        flag: word.flag,
        field: FieldCode(word.short.0),
        start: Register(word.a.0),
        out: Register(word.b.0),
    }
}

/// The `StageWrite` word of `operands`.
#[must_use]
pub(crate) fn stage_write(operands: WriteWord) -> CodeWord {
    CodeWord {
        op: Op::StageWrite,
        flag: operands.flag,
        short: operands.columns,
        a: WordSlot(operands.table.0),
        x: WordSlot(operands.operands.0),
        ..CodeWord::default()
    }
}

/// The operands of a `StageWrite` word.
#[must_use]
pub(crate) const fn stage_write_of(word: CodeWord) -> WriteWord {
    WriteWord {
        flag: word.flag,
        columns: word.short,
        table: MadeTableId(word.a.0),
        operands: PoolAt(word.x.0),
    }
}
