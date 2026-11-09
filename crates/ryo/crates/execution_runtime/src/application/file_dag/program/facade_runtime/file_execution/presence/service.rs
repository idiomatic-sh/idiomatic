//! Deciding which of a file's roots and reached regions cannot have a row or a row error there: each formula is evaluated
//! lazily over the file's own rows, each relation's pending tests in one pass, and the answers are kept for the file.
use crate::application::file_dag::program::facade_runtime::file_execution::FileRun;
use crate::application::file_dag::program::facade_runtime::types::{FilePresence, PresenceGate, RootCandidate, Truth};
use ryo_execution_domain::domain::executable_program::file_demand::types::AbsentRegions;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::presence::service::of_region;
use ryo_execution_domain::domain::presence::types::{FormulaId, RegionPresence, PresenceFormula, PresenceTable, TestId};
use ryo_execution_domain::domain::readiness::types::RegionCount;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_relations::domain::relations::types::Relation;

/// What a presence test reads: the file's rows, the texts its comparisons resolve, and the request's prepared matchers.
struct PresenceRead<'r, 'facts> {
    read: &'r dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    texts: &'r ryo_execution_domain::domain::table_texts::types::Texts<'r>,
    matchers: &'r ryo_execution_domain::domain::matching::types::prepared::value::PreparedMatchers,
}

impl FileRun<'_> {
    /// The run rules' prepared roots whose skip formula fails on the file. With the gate off (a partial read) none is.
    pub(in crate::application::file_dag::program::facade_runtime::file_execution) fn decide_presence<'facts>(
        &mut self,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &ryo_language::domain::symbols::types::Symbols,
        source: ryo_values::domain::files::types::SourceView<'_>,
        gate: PresenceGate,
        candidates: &[RootCandidate],
    ) -> Result<AbsentRegions, OperatingError> {
        let program = self.program.program();
        let mut absent = AbsentRegions::none(RegionCount(program.regions().len()));
        if gate == PresenceGate::Off {
            return Ok(absent);
        }
        let table = program.presence();
        self.presence.reset(table);
        let texts = ryo_execution_domain::domain::table_texts::types::Texts {
            symbols,
            source,
            paths: self.services.request().paths(),
            arenas: &[],
        };
        let tested = PresenceRead {
            read,
            texts: &texts,
            matchers: self.program.matchers().as_ref(),
        };
        for &RootCandidate { region: id, .. } in candidates {
            if absent.presence(id) == RegionPresence::Present
                && self.presence.holds(table, of_region(table, id)?, &tested) == Truth::Fails
            {
                absent.insert(id);
            }
        }
        Ok(absent)
    }
    /// The regions the file's admitted roots reach whose skip formula fails on the file, absent roots aside: each
    /// publishes the empty table when admission reaches it, and releases its counted inputs.
    pub(in crate::application::file_dag::program::facade_runtime::file_execution) fn decide_interior<'facts>(
        &mut self,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
        symbols: &ryo_language::domain::symbols::types::Symbols,
        source: ryo_values::domain::files::types::SourceView<'_>,
        gate: PresenceGate,
        roots: &AbsentRegions,
    ) -> Result<AbsentRegions, OperatingError> {
        let program = self.program.program();
        let mut interior = AbsentRegions::none(RegionCount(program.regions().len()));
        if gate == PresenceGate::On {
            let table = program.presence();
            let texts = ryo_execution_domain::domain::table_texts::types::Texts {
                symbols,
                source,
                paths: self.services.request().paths(),
                arenas: &[],
            };
            let tested = PresenceRead {
                read,
                texts: &texts,
                matchers: self.program.matchers().as_ref(),
            };
            for id in self.execution.reached() {
                let region = program.regions().get(id.0).ok_or(OperatingError::NotRun)?;
                if roots.presence(id) == RegionPresence::Present
                    && region.phase() == ryo_values::domain::plan_parts::types::StepScope::PerFile
                    && self.presence.holds(table, of_region(table, id)?, &tested) == Truth::Fails
                {
                    interior.insert(id);
                }
            }
        }
        self.execution.mark_interior(interior.clone());
        Ok(interior)
    }
}

impl FilePresence {
    /// Forgets the last file's answers, keeping the room.
    fn reset(&mut self, table: &PresenceTable) {
        self.tests.clear();
        self.tests.resize(table.tests.len(), Truth::Unknown);
        self.formulas.clear();
        self.formulas.resize(table.formulas.len(), Truth::Unknown);
    }
    /// Whether a formula holds on the file. It cannot fail: a test whose rows cannot be read, or whose comparison errs on
    /// a row, holds, so the region runs in full and reports the error exactly as it would have.
    fn holds(&mut self, table: &PresenceTable, formula: FormulaId, tested: &PresenceRead<'_, '_>) -> Truth {
        match self.formulas.get(formula.0).copied() {
            Some(Truth::Unknown) => {}
            Some(known @ (Truth::Holds | Truth::Fails)) => return known,
            None => return Truth::Holds,
        }
        let truth = match table.formulas.get(formula.0) {
            Some(PresenceFormula::Test(test)) => self.test(table, *test, tested),
            Some(PresenceFormula::All(parts)) => {
                if parts.iter().all(|part| self.holds(table, *part, tested) == Truth::Holds) {
                    Truth::Holds
                } else {
                    Truth::Fails
                }
            }
            Some(PresenceFormula::Any(parts)) => {
                if parts.iter().any(|part| self.holds(table, *part, tested) == Truth::Holds) {
                    Truth::Holds
                } else {
                    Truth::Fails
                }
            }
            Some(PresenceFormula::Always) | None => Truth::Holds,
        };
        if let Some(slot) = self.formulas.get_mut(formula.0) {
            *slot = truth;
        }
        truth
    }
    /// Whether a test holds, deciding every pending test of its relation in the same pass over the relation's rows.
    fn test(&mut self, table: &PresenceTable, test: TestId, tested: &PresenceRead<'_, '_>) -> Truth {
        match self.tests.get(test.0).copied() {
            Some(Truth::Unknown) => {}
            Some(known @ (Truth::Holds | Truth::Fails)) => return known,
            None => return Truth::Holds,
        }
        let Some(relation) = table.tests.get(test.0).map(|each| each.relation) else {
            return Truth::Holds;
        };
        let mut open: Vec<TestId> = table
            .relations
            .iter()
            .find(|group| group.relation == relation)
            .map_or_else(
                || vec![test],
                |group| {
                    group
                        .tests
                        .iter()
                        .copied()
                        .filter(|each| self.tests.get(each.0) == Some(&Truth::Unknown))
                        .collect()
                },
            );
        if let Ok(rows) = (tested.read)(relation) {
            let mut cells = Vec::new();
            let mut memo = Vec::new();
            for index in ryo_values::domain::rows::service::positions(
                ryo_execution_domain::domain::table_model::service::count_of(&rows),
            ) {
                if open.is_empty() {
                    break;
                }
                let Some(row) = ryo_execution_domain::domain::table_model::service::read_row(&rows, index, &mut cells)
                else {
                    continue;
                };
                let tests = &mut self.tests;
                let scratch = &mut self.scratch;
                open.retain(|each| {
                    let Some(presence) = table.tests.get(each.0) else {
                        return false;
                    };
                    let verdict = ryo_execution_domain::domain::comparing::service::holds_bound(
                        &presence.comparisons,
                        row,
                        ryo_execution_domain::domain::comparing::types::RowShape {
                            kinds: &presence.kinds,
                            names: ryo_execution_domain::domain::operating::types::NameCount(0),
                        },
                        tested.texts,
                        &[],
                        Some(tested.matchers),
                        ryo_execution_domain::domain::matching::types::PatternMatcher(
                            &ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl,
                        ),
                        ryo_execution_domain::domain::comparing::types::DecisionWorkspace {
                            select: &mut *scratch,
                            memo: &mut memo,
                        },
                    );
                    // An error holds the test: the region runs and reports it as the full run would.
                    if matches!(verdict, Ok(ryo_execution_domain::domain::table_model::types::Verdict::Fails)) {
                        true
                    } else {
                        if let Some(slot) = tests.get_mut(each.0) {
                            *slot = Truth::Holds;
                        }
                        false
                    }
                });
            }
            for each in open {
                if let Some(slot) = self.tests.get_mut(each.0) {
                    *slot = Truth::Fails;
                }
            }
        } else {
            // Rows that cannot be read hold every pending test.
            for each in open {
                if let Some(slot) = self.tests.get_mut(each.0) {
                    *slot = Truth::Holds;
                }
            }
        }
        match self.tests.get(test.0) {
            Some(truth) => *truth,
            None => Truth::Holds,
        }
    }
}
