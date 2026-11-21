//! The services of the `places` part: what the walk's two kinds of word share. Whether the executor runs a test; the
//! table a site's index is over; what a body record reaches.

use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{IndexSite, PoolAt, ProgramCode, WordSlot};
use crate::domain::executing::running::types::{WalkPlace, WordBuilt};
use crate::domain::made_tables::reading::service::site_at;
use crate::domain::steps::types::{MadeTableId, TestOperation};

/// Whether the executor runs a test: every comparison of two cells, a set either way, a regular expression and a glob.
pub(crate) const fn tested(test: Option<TestOperation>) -> WordBuilt {
    match test {
        Some(
            TestOperation::Equal
            | TestOperation::NotEqual
            | TestOperation::Less
            | TestOperation::LessOrEqual
            | TestOperation::Greater
            | TestOperation::GreaterOrEqual
            | TestOperation::In
            | TestOperation::NotIn
            | TestOperation::Matches(_)
            | TestOperation::Glob
            | TestOperation::NotGlob,
        ) => WordBuilt::Runs,
        Some(
            TestOperation::StartsWith
            | TestOperation::EndsWith
            | TestOperation::NumericAtLeast
            | TestOperation::NaturalLess
            | TestOperation::Successor,
        )
        | None => WordBuilt::NotYet,
    }
}

/// The table a site's index is over, when its source is a made table.
pub(crate) fn site_table(code: &ProgramCode, site: IndexSite, out: &mut Vec<WalkPlace>) {
    if let Ok(index) = site_at(code, site)
        && index.source.0 == 2
    {
        out.push(WalkPlace::Table(MadeTableId(index.number.0)));
    }
}

/// What a body record reaches: an inline body its program in the asking unit, a pattern in call form its program in its
/// own unit, a pattern in table form its table. Answers the place after the record; none when the pool does not hold it.
pub(crate) fn body(
    code: &ProgramCode,
    at: PoolAt,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> Option<PoolAt> {
    let read = body_at(&code.pool, at)?;
    match read.record {
        BodyRecord::Inline { entry } => out.push(WalkPlace::Program { entry, unit }),
        BodyRecord::Call {
            entry, unit: own, ..
        } => out.push(WalkPlace::Program { entry, unit: own }),
        BodyRecord::Table { index, .. } => site_table(code, index, out),
    }
    Some(read.after)
}
