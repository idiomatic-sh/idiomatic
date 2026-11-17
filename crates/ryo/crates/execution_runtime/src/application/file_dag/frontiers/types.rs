//! The types of the `frontiers` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::coverage::types::{Covered, InapplicableFiles, RefusedFiles};
use ryo_operators::domain::contract::types::WithheldInput;
use ryo_values::domain::files::types::CorpusInput;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::runs::types::Skipped;
use std::sync::Arc;

/// What the files of a phase are scoped by: the corpus, the files each rule read in part, skipped, does not apply to or was
/// refused on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PhaseScoping<'a> {
    /// The files of the request.
    pub(crate) corpus: &'a [CorpusInput],
    /// The files whose relations the rules read in part.
    pub(crate) covered: &'a [Covered],
    /// The files each rule skipped.
    pub(crate) skipped: &'a [Skipped],
    /// The files each rule does not apply to, by rule.
    pub(crate) inapplicable: &'a [InapplicableFiles],
    /// The files each rule was refused on, by rule.
    pub(crate) refused: &'a [RefusedFiles],
}

/// What a rule's frontier is handed over for: its diagnostic, or its observations once the producers each entry depends on
/// published, as the stage proves it.
#[derive(Clone, Copy, Debug)]
pub(crate) enum HandoverPolicy<'a> {
    /// For the diagnostic: decoded permissively.
    Diagnostic,
    /// For the observations: decoded completely, an entry whose producer is unavailable withheld.
    Observations {
        /// Whether each entry's producers published, in frontier order.
        ready: &'a ReadinessProof,
    },
}

/// What the files handed over of one rule's frontier, by entry, decoded once per phase into the phase's symbols.
#[derive(Debug, Default)]
pub(crate) struct HandedOver(pub(crate) Vec<FrontierRows>);

/// One entry of a rule's frontier as the files handed it over.
#[derive(Debug)]
pub(crate) enum FrontierRows {
    /// The rows every file kept, and how completely they cover the rule's files.
    Held {
        /// The rows: a project relation's are the phase's one table, shared with every rule that reads it.
        table: Arc<Table>,
        /// Their coverage.
        coverage: CoverageState,
    },
    /// Withheld: the phase answers the token with the reason it recorded.
    Withheld(WithheldInput),
    /// Not the files' to hand over: a bound observation.
    Elsewhere,
}

/// What the store read and the proof left of one rule's frontier, by entry, before any entry is decoded: no word is held.
#[derive(Debug, Default)]
pub(crate) struct ProvenFrontier(pub(crate) Vec<ProvenEntry>);

/// One frontier entry once the store was read and its proof checked.
#[derive(Debug)]
pub(crate) enum ProvenEntry {
    /// The files kept it and its provenance is proven: its words are read once, where it is decoded.
    Words,
    /// Refused before its words were read, for this reason.
    Refused(ryo_execution_domain::domain::operating::errors::OperatingError),
    /// A project relation's rows, as the phase read them.
    Project(Arc<Table>),
    /// Not the files' to hand over.
    Elsewhere,
}

/// Where a bucket stands in a run: the rule and frontier entry it holds, the rule's first bucket and the bucket itself.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BucketPlace {
    /// The rule whose frontier it holds.
    pub(crate) rule: ryo_execution_domain::domain::coverage::types::RuleIndex,
    /// The frontier entry it holds.
    pub(crate) input: ryo_values::domain::plan_parts::types::InputNumber,
    /// The rule's first bucket.
    pub(crate) first: ryo_values::domain::stored_facts::types::Bucket,
    /// The bucket.
    pub(crate) bucket: ryo_values::domain::stored_facts::types::Bucket,
}

/// A bucket's provenance as a proof reads it: each source, and how many words they run to.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BucketProvenance<'a> {
    /// Each source whose words the bucket holds.
    pub(crate) sources: &'a [crate::outbound::facts::types::BucketSource],
    /// How many words the bucket holds.
    pub(crate) length: crate::outbound::facts::types::KeptWordsLength,
}

/// Whether the files a source proof accounts for are exactly the files the request requires.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FileAccounting {
    /// Every required file is accounted for, and no other.
    Exact,
    /// A required file is missing, or an unrequired one is named.
    Inexact,
}

/// Whether the producer feeding one retained frontier is available to this phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputReadiness {
    /// The producer published; the frontier is read.
    Ready,
    /// The producer is unavailable; the frontier is withheld.
    Unavailable,
}

/// The readiness of each retained frontier of one rule, in frontier order, as the stage's publications prove it: only the stage
/// mints one, so no caller chooses which producers count as published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadinessProof(pub(crate) Vec<InputReadiness>);

/// Why each withheld input of a phase is withheld, by its token.
#[derive(Debug, Default)]
pub(crate) struct Withholdings(
    pub(crate) Vec<ryo_execution_domain::domain::operating::errors::OperatingError>,
);
