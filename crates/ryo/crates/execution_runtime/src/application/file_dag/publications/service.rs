//! The `publications` service: each occurrence's publication slot and install, the arenas they publish into, the named outputs
//! the files kept restored for their consumers, the tables each consumer's bound observations are bound to, and each output
//! released once its last consumer is done.

use crate::application::file_dag::program::preparation::types::{
    ObservationMetadata, OutputRoot,
};
use crate::application::file_dag::frontiers::service::withheld;
use crate::application::file_dag::frontiers::types::{InputReadiness, Withholdings};
use crate::application::file_dag::publications::constants::PUBLICATION_INCOMPLETE;
use crate::application::file_dag::publications::types::{
    ArenaCount, BoundRows, BoundTable, BoundTables, ConsumerCount, CrossRoot,
    FinishState, Publications, PublicationSlot, ReferenceCount, RestoredOutput,
};
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::observation_schedule::types::ResolvedObservationProducer;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::output_coverage::service::output_coverage;
use ryo_execution_domain::domain::output_coverage::types::{InputCoverage, OperandOrdinal};
use ryo_execution_domain::domain::table_dedup::service::kept_once;
use ryo_language::domain::symbols::service::places_mut;
use ryo_language::domain::symbols::types::Symbols;
use ryo_operators::domain::contract::types::WithheldInput;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_relations::domain::relations::types::{CROSS_FILE, CellKind, WorkerNumber};
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::{Input, ObservationBindingId, StepId, StepScope};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::records::types::ObservationIndex;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::table_words::service::decode_complete;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) mod install;
pub(crate) mod restore;

impl<'a> Publications<'a> {
    /// The stage's publications over `arenas`, which it extends and gives back as it found them.
    pub(crate) fn new(metadata: &'a ObservationMetadata, arenas: &'a mut Vec<ArenaBytes>) -> Self {
        let inherited = arenas.len();
        Self {
            metadata,
            inherited: ArenaCount(inherited),
            references: vec![ReferenceCount(0); inherited],
            arenas,
            roots: metadata
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| match output.root {
                            OutputRoot::CrossFile => CrossRoot::Pending,
                            OutputRoot::PerFile | OutputRoot::Unavailable => CrossRoot::Unavailable,
                        })
                        .collect()
                })
                .collect(),
            remaining: metadata
                .outputs
                .iter()
                .map(|outputs| {
                    outputs
                        .iter()
                        .map(|output| ConsumerCount(output.consumers.len()))
                        .collect()
                })
                .collect(),
            finished: vec![FinishState::Pending; metadata.bindings.len()],
            stage: Arc::new(()),
        }
    }

    /// The arenas the stage's texts are in.
    pub(crate) fn arenas(&self) -> &[ArenaBytes] {
        self.arenas
    }

    /// Whether each binding of `rule` has every producer published.
    ///
    /// # Errors
    ///
    /// The publication's refusal for a rule outside the program.
    pub(crate) fn binding_readiness(
        &self,
        rule: RuleIndex,
    ) -> Result<BTreeMap<ObservationBindingId, InputReadiness>, OperatingError> {
        let bindings = self.metadata.bindings.get(rule.0).ok_or_else(incomplete)?;
        Ok(bindings
            .iter()
            .map(|binding| {
                let available = binding.producers.iter().all(|producer| match producer {
                    ResolvedObservationProducer::NoQuery { .. } => false,
                    ResolvedObservationProducer::Output { rule, output } => matches!(
                        self.roots
                            .get(rule.0)
                            .and_then(|roots| roots.get(output_place(*output))),
                        Some(CrossRoot::Restored(_) | CrossRoot::Named { .. })
                    ),
                });
                (
                    binding.input,
                    if available {
                        InputReadiness::Ready
                    } else {
                        InputReadiness::Unavailable
                    },
                )
            })
            .collect())
    }

    /// Reserves an arena for an occurrence of `rule` when the rule publishes anything after the files.
    ///
    /// # Errors
    ///
    /// `NotRun` for a rule outside the program or when the arenas are full.
    pub(crate) fn reserve(&mut self, rule: RuleIndex) -> Result<PublicationSlot, OperatingError> {
        let outputs = self
            .metadata
            .outputs
            .get(rule.0)
            .ok_or(OperatingError::NotRun)?;
        if !outputs
            .iter()
            .any(|output| matches!(output.root, OutputRoot::CrossFile))
        {
            return Ok(PublicationSlot::Unused);
        }
        let worker =
            WorkerNumber(u32::try_from(self.arenas.len()).map_err(|_| OperatingError::NotRun)?);
        if worker == CROSS_FILE {
            return Err(OperatingError::NotRun);
        }
        self.arenas.push(ArenaBytes::default());
        self.references.push(ReferenceCount(0));
        Ok(PublicationSlot::Reserved(worker))
    }

    /// The tables `rule`'s bound observations after the files are bound to, in binding order, each its producers' rows in
    /// producer order, each row once, read in `symbols`: a producer that published nothing withholds its binding, with the
    /// reason recorded in `withholdings`.
    ///
    /// # Errors
    ///
    /// The publication's refusal for a rule outside the program.
    pub(crate) fn bindings(
        &self,
        rule: RuleIndex,
        symbols: &mut Symbols,
        withholdings: &mut Withholdings,
    ) -> Result<BoundTables, OperatingError> {
        let bindings = self.metadata.bindings.get(rule.0).ok_or_else(incomplete)?;
        let mut tables = Vec::with_capacity(bindings.len());
        for binding in bindings
            .iter()
            .filter(|binding| binding.schema.scope == StepScope::CrossFile)
        {
            let members = binding
                .producers
                .iter()
                .map(|producer| match producer {
                    ResolvedObservationProducer::Output { rule, output } => {
                        self.member(*rule, *output, symbols)
                    }
                    ResolvedObservationProducer::NoQuery { .. } => Err(incomplete()),
                })
                .collect::<Result<Vec<_>, _>>();
            let declared = binding.schema.columns.iter().map(|column| column.kind).collect();
            let rows = match members.and_then(|members| united(&members, declared)) {
                Ok((table, coverage)) => BoundRows::Held { table, coverage },
                Err(error) => BoundRows::Withheld(withheld(withholdings, error)),
            };
            tables.push(BoundTable {
                input: binding.input,
                rows,
            });
        }
        Ok(BoundTables(tables))
    }

    /// One producer output's rows and coverage, as it published them.
    fn member(
        &self,
        rule: RuleIndex,
        output: ObservationIndex,
        symbols: &mut Symbols,
    ) -> Result<(Arc<Table>, CoverageState), OperatingError> {
        match self
            .roots
            .get(rule.0)
            .and_then(|outputs| outputs.get(output_place(output)))
        {
            Some(CrossRoot::Restored(restored)) => restored
                .decoded
                .get_or_init(|| decoded(restored, symbols))
                .clone()
                .map(|table| (table, restored.coverage.clone())),
            Some(CrossRoot::Named { publication, .. }) => Ok((
                Arc::clone(&publication.rows),
                publication.coverage.clone(),
            )),
            Some(CrossRoot::Pending | CrossRoot::Unavailable) | None => Err(incomplete()),
        }
    }
}

impl Drop for Publications<'_> {
    fn drop(&mut self) {
        // Every install is the invocation's own: the inherited arenas go back without renumbering their slots.
        self.roots.clear();
        self.arenas.truncate(self.inherited.0);
    }
}

/// The rows a bound observation of `place`'s rule answers, or the token of why they are withheld.
pub(crate) fn held_observation(bound: &BoundTables, place: BoundObservationPlace) -> HeldRows<'_> {
    match bound.0.iter().find(|table| table.input == place.input) {
        Some(BoundTable {
            rows: BoundRows::Held { table, .. },
            ..
        }) => HeldRows::Held(table),
        Some(BoundTable {
            rows: BoundRows::Withheld(token),
            ..
        }) => HeldRows::Withheld(*token),
        None => HeldRows::Withheld(WithheldInput(u32::MAX)),
    }
}

/// The coverage of a bound observation's rows, when they are held.
pub(crate) fn bound_coverage(bound: &BoundTables, input: ObservationBindingId) -> Option<&CoverageState> {
    bound.0.iter().find_map(|table| match table {
        BoundTable {
            input: found,
            rows: BoundRows::Held { coverage, .. },
        } if *found == input => Some(coverage),
        BoundTable { .. } => None,
    })
}

/// A restored output's rows in the stage's symbols, checked against the columns its producer makes.
fn decoded(restored: &RestoredOutput, symbols: &mut Symbols) -> Result<Arc<Table>, OperatingError> {
    let places = places_mut(symbols);
    let before = places.len();
    let table = decode_complete(&restored.words.0, places).map_err(|error| {
        OperatingError::ObservationIncomplete {
            reason: error.to_string(),
        }
    });
    let table = match table {
        Ok(table) if table.kinds.as_slice() == restored.kinds.as_ref() => Ok(table),
        Ok(_) => Err(OperatingError::ShapeMismatch),
        Err(error) => Err(error),
    };
    if table.is_err() {
        places_mut(symbols).truncate(before);
    }
    table.map(Arc::new)
}

/// The members' rows one after another, each row once, the first kept, with the coverage a union of them answers.
pub(crate) fn united(
    members: &[(Arc<Table>, CoverageState)],
    declared: Vec<CellKind>,
) -> Result<(Table, CoverageState), OperatingError> {
    // A binding no producer feeds is authoritatively empty, of its declared columns.
    let mut table = Table {
        kinds: members.first().map_or(declared, |(first, _)| first.kinds.clone()),
        ..Table::default()
    };
    for (member, _) in members {
        if member.kinds != table.kinds {
            return Err(OperatingError::ShapeMismatch);
        }
        table.cells.extend_from_slice(&member.cells);
        table.count.0 = table.count.0.saturating_add(member.count.0);
    }
    kept_once(&mut table);
    // Two members at a time, as the union of each pair of them answers.
    let union = Operation::Union {
        left: Input::Step(StepId(0)),
        right: Input::Step(StepId(1)),
    };
    let mut coverage = members
        .first()
        .map_or(CoverageState::Complete, |(_, coverage)| coverage.clone());
    for (_, next) in members.iter().skip(1) {
        let inputs = [
            InputCoverage {
                ordinal: OperandOrdinal(0),
                kinds: &table.kinds,
                coverage,
                lineage: Lineage::Mixed,
            },
            InputCoverage {
                ordinal: OperandOrdinal(1),
                kinds: &table.kinds,
                coverage: next.clone(),
                lineage: Lineage::Mixed,
            },
        ];
        coverage = output_coverage(&union, &table.kinds, &inputs)?;
    }
    Ok((table, coverage))
}

/// The place of an output among its rule's outputs.
pub(crate) fn output_place(output: ObservationIndex) -> usize {
    usize::try_from(output.0).unwrap_or(usize::MAX)
}

/// The place of an arena among the stage's arenas.
pub(crate) fn worker_place(worker: WorkerNumber) -> usize {
    usize::try_from(worker.0).unwrap_or(usize::MAX)
}

/// The refusal of an unavailable or malformed publication.
pub(crate) fn incomplete() -> OperatingError {
    OperatingError::ObservationIncomplete {
        reason: PUBLICATION_INCOMPLETE.into(),
    }
}
