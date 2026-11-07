//! The request and answer of the operators capability: one borrowed call in, one pure meaning out.

use crate::domain::contract::types::SourceByteLength;
use crate::domain::equality_join::types::JoinInput;
use crate::domain::ordered_existence::types::{
    OrderedExistenceDriver, OrderedExistenceScratch, PartitionExtremum,
};
use crate::domain::ordered_match::types::OrderedMatchDriver;
use crate::domain::capture::types::{CaptureOutput, CaptureSlots, SlotPattern, SlotSubject};
use crate::domain::replace::types::{ReplacedText, ReplacementText};
use crate::domain::select::types::{PredicateOutcome, RowSelection};
use crate::domain::words::types::WordScratch;
use ryo_relations::domain::relations::types::{Count, Integer};
use ryo_values::domain::row_texts::types::CellText;
use crate::domain::reduction::types::{
    AggregationCheckOrder, AggregationHash, AggregationPlan, DigestDatum, DigestValues,
    Finishing, GroupColumns, GroupCounts, GroupOutput, HashScratch, StreamingReduction, ValueHash,
};
use crate::domain::set_rows::types::SetInput;
use crate::domain::source_matches::types::{SourceMatchesInput, TextRowDriver};
use crate::domain::words::types::WordOutput;

/// The result of one pure operator meaning, before engine publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperatorAnswer {
    /// Caller-owned scratch was filled.
    Written,
    /// A predicate accepted this row; an empty projection keeps the original row.
    Selected(RowSelection),
    /// The first matching resolved rank, or no matching layer.
    Rank(Option<ryo_relations::domain::relations::types::Integer>),
    /// Stable text-aware digest hash.
    ValueHash(ValueHash),
    /// Checked half-open span endpoints.
    SpanBounds([ryo_relations::domain::relations::types::Value; 2]),
    /// A composed half-open span, including a declared absent span.
    SpanBetween(ryo_relations::domain::relations::types::Value),
}

/// A borrowed request for one pure operator meaning. Storage and publication stay with the engine.
pub enum OperatorCall<'a> {
    /// Reduce a prepared key order into first-occurrence group counts.
    ReduceCount {
        table: &'a ryo_values::domain::rows::types::Table,
        keys: GroupColumns<'a>,
        distinct: Option<ryo_values::domain::plan_parts::types::ColumnIndex>,
        order: &'a [ryo_values::domain::rows::types::RowIndex],
        values: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
        target: &'a mut dyn GroupOutput,
    },
    /// Reduce distinct text-aware hashes in the prepared key order.
    ReduceDigest {
        table: &'a ryo_values::domain::rows::types::Table,
        keys: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        over: ryo_values::domain::plan_parts::types::ColumnIndex,
        order: &'a [ryo_values::domain::rows::types::RowIndex],
        hashes: HashScratch<'a>,
        hasher: &'a dyn DigestValues,
        target: &'a mut dyn GroupOutput,
    },
    /// Hash resolved text and typed scalar/list values without acquiring their backing storage.
    HashValue { datum: DigestDatum<'a> },
    /// Check aggregation columns in the original collected or streamed order.
    CheckAggregation {
        program: AggregationPlan<'a>,
        kinds: &'a [ryo_relations::domain::relations::types::CellKind],
        order: AggregationCheckOrder,
        target: &'a mut Vec<ryo_relations::domain::relations::types::CellKind>,
    },
    /// Reduce one already admitted row into engine-owned streaming state.
    StreamAggregation {
        program: AggregationPlan<'a>,
        row: &'a [ryo_relations::domain::relations::types::Value],
        key: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
        driver: &'a mut dyn StreamingReduction,
    },
    /// Finish an engine-owned streaming reduction without retaining its state.
    FinishAggregation {
        counts: GroupCounts<'a>,
        hashes: &'a mut Vec<AggregationHash>,
        digest: Finishing,
        target: &'a mut dyn GroupOutput,
    },

    /// Apply an already evaluated predicate and the authored kept-column policy.
    Select {
        row: &'a [ryo_relations::domain::relations::types::Value],
        kept: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        holds: PredicateOutcome,
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Split a resolved text using the language-independent word policy.
    Words {
        text: CellText<'a>,
        word: WordScratch<'a>,
        each: &'a mut dyn WordOutput,
    },
    /// Enumerate nonempty path-slot captures as UTF-8 byte ranges in encounter order.
    Capture {
        pattern: SlotPattern<'a>,
        text: SlotSubject<'a>,
        slots: CaptureSlots<'a>,
        each: &'a mut dyn CaptureOutput,
    },
    /// Return the first matching resolved text-set layer.
    Rank {
        text: CellText<'a>,
        layers: &'a [Vec<ryo_values::domain::plan_parts::types::LiteralText>],
    },
    /// Replace all matches literally; matcher acquisition and storage belong to the engine.
    Replace {
        regex: &'a ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex,
        text: CellText<'a>,
        replacement: ReplacementText<'a>,
        target: ReplacedText<'a>,
    },
    /// Format an already reduced group; reduction storage belongs to the engine.
    Aggregate {
        row: &'a [ryo_relations::domain::relations::types::Value],
        keys: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        count: Count,
        digest: Option<Integer>,
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Pair each prepared open key with the first later close key in FIFO order.
    OrderedMatch {
        output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
        driver: &'a mut dyn OrderedMatchDriver,
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Match a previously checked physical source interval and format its occurrences.
    SourceMatches {
        input: SourceMatchesInput<'a>,
        driver: &'a mut dyn TextRowDriver,
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },

    /// Flag exactly the source rows dominated by a strict same-column ordered selfjoin.
    /// The original downstream projection and antijoin remain engine-scheduled operations.
    OrderedExistence {
        partition: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        order: ryo_values::domain::plan_parts::types::ColumnIndex,
        extremum: PartitionExtremum,
        driver: &'a mut dyn OrderedExistenceDriver,
        scratch: &'a mut OrderedExistenceScratch,
    },
    /// Apply one set or antijoin row meaning over engine-owned indexes and scratch.
    SetRow {
        /// Borrowed row and the already acquired physical membership boundary.
        input: SetInput<'a>,
    },
    /// Join one left row with the engine's ordered, already matching candidates.
    EqualityJoin {
        /// Borrowed values or deferred file-local position slots, selected by the engine.
        input: JoinInput<'a>,
    },
    /// Copy selected columns in plan order, preserving duplicates.
    Project {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Ordered column indices.
        columns: &'a [ryo_values::domain::plan_parts::types::ColumnIndex],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Append a resolved row suffix without acquiring or publishing it.
    Append {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Resolved suffix.
        appended: &'a [ryo_relations::domain::relations::types::Value],
        /// Caller-owned scratch output.
        target: &'a mut Vec<ryo_relations::domain::relations::types::Value>,
    },
    /// Read checked endpoints of one span cell.
    SpanBounds {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Declared span column.
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    },
    /// Compose the start of one span with the end of another.
    SpanBetween {
        /// Input row.
        row: &'a [ryo_relations::domain::relations::types::Value],
        /// Left span column.
        start: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Right span column.
        end: ryo_values::domain::plan_parts::types::ColumnIndex,
        /// Complete source byte length validated by the engine.
        source_len: SourceByteLength,
    },
}
