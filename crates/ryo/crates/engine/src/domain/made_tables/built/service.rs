//! The services of the `built` part: the one answer to "does the fill side run this stage word, this fill?". The loop's
//! load asks it of everything a rule can reach, and a pipeline asks it of its own words before its first batch.

use crate::domain::code::constants::{BUCKET, NO_SLOT};
use crate::domain::code::filling::service::{
    stage_call_of, stage_keep_of, stage_latest_of, stage_probe_of, stage_product_of, stage_read_of,
    stage_scan_of,
};
use crate::domain::code::small_codes::service::{comparison_of, meaning_of};
use crate::domain::code::types::{IndexSite, ProgramCode};
use crate::domain::filled::types::FillRuns;
use crate::domain::made_tables::built::types::FillPart;
use crate::domain::made_tables::carrying::service::stage_kind;
use crate::domain::made_tables::fills::service::kind_of;
use crate::domain::made_tables::pipelines::types::StageKind;
use crate::domain::made_tables::reading::service::{latest_site, site_at, site_source, source_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::stages::calling::service::call_runs;
use crate::domain::stages::testing::service::test_runs;

/// Whether the fill side runs a part of the code: a stage word of a pipeline, or a made table's fill. The one answer:
/// a pipeline asks it of each of its words before its first batch, a fill's record is read through the same `kind_of`,
/// and a `Keep` and a `Call` ask the same `test_runs` and `call_runs` before they run, so a stage or a fill built here
/// answers `Runs` here and cannot be left off a list. A word that is no stage of a pipeline, or one the code's readers
/// cannot read, is `NotYet`. It answers for the word alone: a table read while its own fill runs is a fault of the run.
#[must_use]
pub(crate) fn runs(code: &ProgramCode, part: FillPart) -> FillRuns {
    let word = match part {
        FillPart::Fill(fill) => {
            return match kind_of(fill) {
                Ok(_) => FillRuns::Runs,
                Err(_) => FillRuns::NotYet,
            };
        }
        FillPart::Stage(word) => word,
    };
    let Some(kind) = stage_kind(word.op) else {
        return FillRuns::NotYet;
    };
    // A stage that reads a table while it is written, or an index over the nodes that sorts each key's rows, is not run.
    let read = |source: ReadSource| match source {
        ReadSource::Nodes(_) | ReadSource::Filled(_) | ReadSource::Made(_) => FillRuns::Runs,
        ReadSource::Written(_) => FillRuns::NotYet,
    };
    let indexed = |site: IndexSite| match site_at(code, site) {
        Ok(index) => match site_source(code, index) {
            Ok(ReadSource::Nodes(_)) if index.ordered != NO_SLOT => FillRuns::NotYet,
            Ok(source) => read(source),
            Err(_) => FillRuns::NotYet,
        },
        Err(_) => FillRuns::NotYet,
    };
    match kind {
        StageKind::Scan => {
            let stage = stage_scan_of(word);
            if stage.flag.0 & BUCKET.0 == 0 {
                read(source_of(stage.flag, stage.read))
            } else {
                indexed(IndexSite(stage.read.0))
            }
        }
        StageKind::Product => {
            let stage = stage_product_of(word);
            read(source_of(stage.flag, stage.read))
        }
        StageKind::Read => {
            let stage = stage_read_of(word);
            read(source_of(stage.flag, stage.read))
        }
        StageKind::Probe => indexed(stage_probe_of(word).index),
        StageKind::Latest => match latest_site(code, stage_latest_of(word).operands) {
            Ok(site) => indexed(site),
            Err(_) => FillRuns::NotYet,
        },
        StageKind::Keep => {
            let stage = stage_keep_of(word);
            match comparison_of(stage.comparison, stage.regex) {
                Some(test) => test_runs(test),
                None => FillRuns::NotYet,
            }
        }
        StageKind::Call => {
            let stage = stage_call_of(word);
            match meaning_of(stage.meaning, stage.regex, stage.flag) {
                Some(meaning) => call_runs(meaning),
                None => FillRuns::NotYet,
            }
        }
        StageKind::Use | StageKind::Walk | StageKind::Write => FillRuns::Runs,
        StageKind::Render => FillRuns::NotYet,
    }
}
