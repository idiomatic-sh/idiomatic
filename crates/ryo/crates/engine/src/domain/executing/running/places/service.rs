//! The services of the `places` part: what the walk's two kinds of word share. Whether the executor runs a test; the
//! table a site's index is over; what a body record reaches.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::records::service::body_at;
use crate::domain::code::records::types::BodyRecord;
use crate::domain::code::types::{IndexSite, PoolAt, ProgramCode, WordSlot};
use crate::domain::executing::running::types::{WalkPlace, WordBuilt};
use crate::domain::made_tables::reading::service::{site_at, site_source};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::steps::types::{CallMeaning, TestOperation};

/// Whether the executor runs a test: every comparison of two cells, a set either way, a regular expression, a glob, a
/// prefix and a suffix.
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
            | TestOperation::NotGlob
            | TestOperation::StartsWith
            | TestOperation::EndsWith,
        ) => WordBuilt::Runs,
        Some(
            TestOperation::NumericAtLeast | TestOperation::NaturalLess | TestOperation::Successor,
        )
        | None => WordBuilt::NotYet,
    }
}

/// Whether the executor runs a call of a per-row meaning: a call word of the loop and a call stage of a pipeline are
/// one kernel (`stages::calling::service::call_batch`), which runs these four.
pub(crate) const fn called(meaning: Option<CallMeaning>) -> WordBuilt {
    match meaning {
        Some(
            CallMeaning::SpanBounds
            | CallMeaning::Replace(_)
            | CallMeaning::Capture
            | CallMeaning::SourceMatches(..),
        ) => WordBuilt::Runs,
        Some(
            CallMeaning::SpanBetween
            | CallMeaning::Words
            | CallMeaning::Rank
            | CallMeaning::Concat
            | CallMeaning::Lower
            | CallMeaning::FitsRendered,
        )
        | None => WordBuilt::NotYet,
    }
}

/// Whether the fill side reads a site's index, and the table the index is over when its source is a made table. An
/// index over a table read while its fill writes it, and an ordered index over the node rows, it does not build yet.
pub(crate) fn site_table(
    code: &ProgramCode,
    site: IndexSite,
    out: &mut Vec<WalkPlace>,
) -> WordBuilt {
    let Ok(index) = site_at(code, site) else {
        return WordBuilt::NotYet;
    };
    match site_source(code, index) {
        Ok(ReadSource::Made(table)) => {
            out.push(WalkPlace::Table(table));
            WordBuilt::Runs
        }
        Ok(ReadSource::Nodes(_)) if index.ordered != NO_SLOT => WordBuilt::NotYet,
        Ok(ReadSource::Nodes(_) | ReadSource::Filled(_)) => WordBuilt::Runs,
        Ok(ReadSource::Written(_)) | Err(_) => WordBuilt::NotYet,
    }
}

/// What a body record reaches: an inline body its program in the asking unit, a pattern in call form its program in its
/// own unit, a pattern in table form its table. Answers the place after the record; none when the pool does not hold it
/// or the fill side does not read the table-form pattern's index.
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
        BodyRecord::Table { index, .. } => {
            if site_table(code, index, out) == WordBuilt::NotYet {
                return None;
            }
        }
    }
    Some(read.after)
}
