//! The findings each operator's fixtures state, one module per operator under `tests/fixtures`.

#![expect(
    clippy::inline_modules,
    reason = "each operator's expected rows are a file of their own under tests/fixtures, which `--bless` writes; `include!` makes each one a module here, and the module spec declares no file by `#[path]` but a file's own test"
)]

use ryo_language::inbound::use_cases::parse::fixtures::expected::Cell;
use ryo_values::domain::query_operators::types::QueryOperator;

/// One finding the rule of an operator gives: what it is about, and the cells of its key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Finding {
    /// The cell the finding is about, written first in its row.
    pub(super) subject: Cell,
    /// The cells that tell this finding from another about the same subject.
    pub(super) key: Vec<Cell>,
}

/// A finding about `subject` with `key`.
pub(super) fn finding(subject: impl Into<Cell>, key: Vec<impl Into<Cell>>) -> Finding {
    Finding {
        subject: subject.into(),
        key: key.into_iter().map(Into::into).collect(),
    }
}

/// A cell of a key as the rows of a rule write it, one row to a line: a tab and a line break are `\t`, `\n` and `\r`.
pub(super) fn escaped(text: &str) -> String {
    text.replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

/// The findings the rule of `operator` gives over the `function` fixture, in any order, as the operator's
/// `tests/fixtures/operators/<item>/expected.rs` states them.
pub(super) fn expected(operator: QueryOperator) -> Vec<Finding> {
    match operator {
        QueryOperator::Replace => replace::expected(),
        QueryOperator::SpanBetween => span_between::expected(),
        QueryOperator::SourceMatches => source_matches::expected(),
        QueryOperator::RenderSyntax => {
            unreachable!("a language without semantic rendering does not register this fixture")
        }
        QueryOperator::FitsRendered => fits_rendered::expected(),
        QueryOperator::SpanBounds => span_bounds::expected(),
        QueryOperator::OrderedMatch => ordered_match::expected(),
        QueryOperator::OrderedDfs => ordered_dfs::expected(),
        QueryOperator::Antijoin => antijoin::expected(),
        QueryOperator::Axis => axis::expected(),
        QueryOperator::Capture => capture::expected(),
        QueryOperator::Closure => closure::expected(),
        QueryOperator::Cycles => cycles::expected(),
        QueryOperator::Difference => difference::expected(),
        QueryOperator::Extend => extend::expected(),
        QueryOperator::GroupCount => group_count::expected(),
        QueryOperator::GroupDigest => group_digest::expected(),
        QueryOperator::Intersection => intersection::expected(),
        QueryOperator::Join => join::expected(),
        QueryOperator::Project => project::expected(),
        QueryOperator::Rank => rank::expected(),
        QueryOperator::Rows => rows_operator::expected(),
        QueryOperator::Select => select::expected(),
        QueryOperator::Union => union::expected(),
        QueryOperator::Words => words::expected(),
    }
}

/// The findings `tests/fixtures/operators/antijoin/expected.rs` states for the `antijoin` operator.
mod antijoin {
    include!("../../../../../tests/fixtures/operators/antijoin/expected.rs");
}

/// The findings `tests/fixtures/operators/rows/expected.rs` states for the `rows` operator.
mod rows_operator {
    include!("../../../../../tests/fixtures/operators/rows/expected.rs");
}

/// The findings `tests/fixtures/operators/axis/expected.rs` states for the `axis` operator.
mod axis {
    include!("../../../../../tests/fixtures/operators/axis/expected.rs");
}

/// The findings `tests/fixtures/operators/capture/expected.rs` states for the `capture` operator.
mod capture {
    include!("../../../../../tests/fixtures/operators/capture/expected.rs");
}

/// The findings `tests/fixtures/operators/closure/expected.rs` states for the `closure` operator.
mod closure {
    include!("../../../../../tests/fixtures/operators/closure/expected.rs");
}

/// The findings `tests/fixtures/operators/cycles/expected.rs` states for the `cycles` operator.
mod cycles {
    include!("../../../../../tests/fixtures/operators/cycles/expected.rs");
}

/// The findings `tests/fixtures/operators/difference/expected.rs` states for the `difference` operator.
mod difference {
    include!("../../../../../tests/fixtures/operators/difference/expected.rs");
}

/// The findings `tests/fixtures/operators/extend/expected.rs` states for the `extend` operator.
mod extend {
    include!("../../../../../tests/fixtures/operators/extend/expected.rs");
}

/// The findings `tests/fixtures/operators/group_count/expected.rs` states for the `group-count` operator.
mod group_count {
    include!("../../../../../tests/fixtures/operators/group_count/expected.rs");
}

/// The findings `tests/fixtures/operators/group_digest/expected.rs` states for the `group-digest` operator.
mod group_digest {
    include!("../../../../../tests/fixtures/operators/group_digest/expected.rs");
}

/// The findings `tests/fixtures/operators/intersection/expected.rs` states for the `intersection` operator.
mod intersection {
    include!("../../../../../tests/fixtures/operators/intersection/expected.rs");
}

/// The findings `tests/fixtures/operators/join/expected.rs` states for the `join` operator.
mod join {
    include!("../../../../../tests/fixtures/operators/join/expected.rs");
}

/// The findings `tests/fixtures/operators/project/expected.rs` states for the `project` operator.
mod project {
    include!("../../../../../tests/fixtures/operators/project/expected.rs");
}

/// The findings `tests/fixtures/operators/rank/expected.rs` states for the `rank` operator.
mod rank {
    include!("../../../../../tests/fixtures/operators/rank/expected.rs");
}

/// The findings `tests/fixtures/operators/select/expected.rs` states for the `select` operator.
mod select {
    include!("../../../../../tests/fixtures/operators/select/expected.rs");
}

/// The findings `tests/fixtures/operators/union/expected.rs` states for the `union` operator.
mod union {
    include!("../../../../../tests/fixtures/operators/union/expected.rs");
}

/// The findings `tests/fixtures/operators/words/expected.rs` states for the `words` operator.
mod words {
    include!("../../../../../tests/fixtures/operators/words/expected.rs");
}

/// Literal replacement fixture, independent expected transformed names.
mod replace {
    include!("../../../../../tests/fixtures/operators/replace/expected.rs");
}

/// Independent byte span projection fixture.
mod span_between {
    include!("../../../../../tests/fixtures/operators/span_between/expected.rs");
}

/// Independent physical occurrence spans; the duplicate nested occurrence is one row, as rows of an identical rule, subject and key collapse.
mod source_matches {
    include!("../../../../../tests/fixtures/operators/source_matches/expected.rs");
}

mod fits_rendered {
    include!("../../../../../tests/fixtures/operators/fits_rendered/expected.rs");
}

mod span_bounds {
    include!("../../../../../tests/fixtures/operators/span_bounds/expected.rs");
}

/// Ordered request/reply fixture, preserving explicit output projection.
mod ordered_match {
    include!("../../../../../tests/fixtures/operators/ordered_match/expected.rs");
}

/// The ordered DFS fixture's complete expected visits.
mod ordered_dfs {
    include!("../../../../../tests/fixtures/operators/ordered_dfs/expected.rs");
}
