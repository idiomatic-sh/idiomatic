//! The services of the `filling` part: whether the fill side runs a stage word of a pipeline and a table's fill, and what
//! each reaches. The stages are listed as the fill side runs them at its frozen head (`claude/engine-x2` at 3e01ab9d7);
//! a fill kind it does not run is its own to say (`fills::service::fill_at` refuses it). The fill side is to give one
//! function that answers both, which replaces the list here.

use crate::domain::code::constants::BUCKET;
use crate::domain::code::filling::service::{
    stage_keep_of, stage_latest_of, stage_probe_of, stage_product_of, stage_scan_of, stage_use_of,
};
use crate::domain::code::small_codes::service::comparison_of;
use crate::domain::code::types::{CodeEntry, CodeWord, IndexSite, Op, ProgramCode, WordSlot};
use crate::domain::executing::running::places::service::{body, site_table, tested};
use crate::domain::executing::running::types::{WalkPlace, WordBuilt};
use crate::domain::made_tables::fills::service::{fill_at, input_at};
use crate::domain::made_tables::fills::types::FillKind;
use crate::domain::made_tables::reading::service::{latest_site, source_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::needs::service::tables_asked;
use crate::domain::steps::types::MadeTableId;

/// The table a stage reads as a source, when it is one.
fn source_table(source: ReadSource, out: &mut Vec<WalkPlace>) {
    match source {
        ReadSource::Made(table) | ReadSource::Written(table) => out.push(WalkPlace::Table(table)),
        ReadSource::Nodes(_) | ReadSource::Filled(_) => {}
    }
}

/// Whether the fill side runs a stage word of a pipeline, and what the stage reaches.
pub(crate) fn stage_word(
    code: &ProgramCode,
    word: CodeWord,
    unit: WordSlot,
    out: &mut Vec<WalkPlace>,
) -> WordBuilt {
    match word.op {
        Op::End | Op::StageRead | Op::StageWalk | Op::StageWrite => WordBuilt::Runs,
        Op::StageScan => {
            let stage = stage_scan_of(word);
            if stage.flag.0 & BUCKET.0 == 0 {
                source_table(source_of(stage.flag, stage.read), out);
            } else {
                site_table(code, IndexSite(stage.read.0), out);
            }
            WordBuilt::Runs
        }
        Op::StageProduct => {
            let stage = stage_product_of(word);
            source_table(source_of(stage.flag, stage.read), out);
            WordBuilt::Runs
        }
        Op::StageProbe => {
            site_table(code, stage_probe_of(word).index, out);
            WordBuilt::Runs
        }
        Op::StageLatest => {
            if let Ok(site) = latest_site(code, stage_latest_of(word).operands) {
                site_table(code, site, out);
            }
            WordBuilt::Runs
        }
        Op::StageKeep => {
            let stage = stage_keep_of(word);
            tested(comparison_of(stage.comparison, stage.regex))
        }
        Op::StageUse => match body(code, stage_use_of(word).body, unit, out) {
            Some(_) => WordBuilt::Runs,
            None => WordBuilt::NotYet,
        },
        // The fill side answers these as not built.
        Op::StageCall | Op::StageRender => WordBuilt::NotYet,
        // A word of a step program is no stage.
        Op::EachKind
        | Op::EachMatchNode
        | Op::EachMatchFilled
        | Op::EachMatchFilledMany
        | Op::EachMatchMade
        | Op::EachRange
        | Op::EachRowNode
        | Op::EachRowFilled
        | Op::EachRowMade
        | Op::Children
        | Op::ChildrenOfField
        | Op::Axis
        | Op::Within
        | Op::Nearest
        | Op::NearestDerived
        | Op::Closure
        | Op::ReadNode
        | Op::ReadFilled
        | Op::ReadMade
        | Op::Test
        | Op::TestEqualConstant
        | Op::TestNotEqualConstant
        | Op::TestEqualRegister
        | Op::TestNotEqualRegister
        | Op::TestInConstants
        | Op::Call
        | Op::Render
        | Op::Let
        | Op::Exists
        | Op::Or
        | Op::Fork
        | Op::Yield
        | Op::Latest
        | Op::Emit => WordBuilt::NotYet,
    }
}

/// The unit of a table when the fill side does not run its fill; none when it does. What the fill reaches is pushed: its
/// input's pipelines, and the tables its own need may ask.
pub(crate) fn fill(
    code: &ProgramCode,
    table: MadeTableId,
    out: &mut Vec<WalkPlace>,
) -> Option<WordSlot> {
    let Some(written) = code
        .tables
        .get(usize::try_from(table.0).unwrap_or(usize::MAX))
    else {
        return Some(WordSlot(0));
    };
    let unit = written.unit;
    let mut asked = Vec::new();
    tables_asked(code, written.need, &mut asked);
    out.extend(asked.into_iter().map(WalkPlace::Table));
    match fill_at(code, table).map(|held| (held.kind, held.inputs)) {
        Ok((FillKind::Constant, _)) => None,
        Ok((FillKind::Rows | FillKind::Count | FillKind::Sorted | FillKind::Ends, inputs)) => {
            match input_at(code, inputs) {
                Ok(input) => {
                    out.extend(input.entries.iter().map(|entry| WalkPlace::Pipeline {
                        entry: CodeEntry(entry.0),
                        unit,
                    }));
                    None
                }
                Err(_) => Some(unit),
            }
        }
        // An ordered match has two inputs, one after the other in its record.
        Ok((FillKind::Match, inputs)) => {
            let both = input_at(code, inputs)
                .and_then(|left| input_at(code, left.after).map(|right| [left, right]));
            match both {
                Ok(sides) => {
                    for side in sides {
                        out.extend(side.entries.iter().map(|entry| WalkPlace::Pipeline {
                            entry: CodeEntry(entry.0),
                            unit,
                        }));
                    }
                    None
                }
                Err(_) => Some(unit),
            }
        }
        Err(_) => Some(unit),
    }
}
