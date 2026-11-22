//! The types of the `needs` concept: a decision, and what a formula's leaves ask of the engine.

use crate::domain::code::types::{NeedSite, RelationNumber, WordSlot};
use crate::domain::steps::types::MadeTableId;
use ryo_relations::domain::relations::types::KindBits;

/// Whether a unit may have a row in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NeedVerdict {
    /// It may: the unit is evaluated.
    Holds,
    /// It cannot: the unit is never evaluated in this file.
    Empty,
}

/// What a formula's leaves ask of the engine, and where a site's decision is kept for the file. The formula is walked
/// here; the engine answers the three leaves that read the file, and may make a table to answer the third.
pub(crate) trait NeedFacts {
    /// The decision kept for a site in this file, when it was decided.
    fn kept(&self, site: NeedSite) -> Option<NeedVerdict>;

    /// Keeps a site's decision for the file.
    fn keep(&mut self, site: NeedSite, verdict: NeedVerdict);

    /// Whether the file holds a node of the kind.
    fn kind_present(&mut self, kind: KindBits) -> NeedVerdict;

    /// Whether the relation has a row in the file, with the constant in the column when both are given (`NO_SLOT` for
    /// none).
    fn relation_row(
        &mut self,
        relation: RelationNumber,
        column: WordSlot,
        constant: WordSlot,
    ) -> NeedVerdict;

    /// Whether the table, made now when its own need holds, has a row.
    fn table_row(&mut self, table: MadeTableId) -> NeedVerdict;

    /// Whether the scanned file's path passes the test (`comparison` and `regex` as a need record holds them) against
    /// the constant. It costs nothing: the path is the file's own.
    fn path_passes(&mut self, _comparison: WordSlot, _regex: WordSlot, _constant: WordSlot) -> NeedVerdict {
        NeedVerdict::Holds
    }

    /// What the terms of a site say of it before its formula is walked, when the engine keeps the masks of its needs:
    /// `Empty` when they leave it no way to hold in the file, `Holds` when they are its whole formula and hold. None:
    /// the walk decides.
    fn masked(&mut self, _site: NeedSite) -> Option<NeedVerdict> {
        None
    }

    /// The file's answer to the record at a place, when the record is a term the engine keeps the answer of. None:
    /// the walk asks the record's own leaf.
    fn leaf(&mut self, _at: WordSlot) -> Option<NeedVerdict> {
        None
    }

    /// Whether deciding the record at a place may cost a fill, whatever the file, when the engine keeps that: `Holds`
    /// it may, `Empty` it costs nothing. None: the walk finds it from the record.
    fn record_cost(&self, _at: WordSlot) -> Option<NeedVerdict> {
        None
    }
}
