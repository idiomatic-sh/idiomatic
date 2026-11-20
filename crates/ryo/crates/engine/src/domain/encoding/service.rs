//! The services of the `encoding` concept: a lowered pack written as code. This is the straight encoding: each step and stage
//! is one word of its own code (a use of a pattern in table form is a probe and its reads), every operand through the code's
//! constructors, nothing specialised. It holds two things the lowering has already proved, as its backstop: no count is cut to
//! fit a width, and a file-sized word stands only first in a rule's branch, first in a pipeline as a `Scan`, or as a `Product`.

use super::bodies::service::entries_written;
use super::needs::service::{assembled, need_site};
use super::programs::service::program_entry;
use super::sites::service::{past, slot_of};
use super::tables::service::reached_written;
use super::types::{Encoding, ProgramRole, TableProgress, UnitPoints};
use super::units::service::{begin_unit, end_unit};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::controlling::service::end;
use crate::domain::code::types::{
    BranchCode, NeedSite, ProgramCode, RuleCode, Short, StateCounts, TableCode, WordSlot,
};
use crate::domain::code::types::{MeasuredForm, ShapeChoice, UnitKey};
use crate::domain::lowered::types::{LoweredPack, LoweredRule};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::PastWidth;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::collections::{HashMap, HashSet};

/// An encoding of `pack` with nothing written but the word no entry names: entry 0 holds an `End`, so no entry is 0.
fn started(pack: &LoweredPack) -> Encoding<'_> {
    let unwritten = TableCode {
        fill: Short(0),
        operands: NO_SLOT,
        need: NeedSite(NO_SLOT.0),
    };
    Encoding {
        pack,
        code: ProgramCode {
            words: vec![end()],
            points: vec![NO_SLOT],
            relations: pack.reads.clone(),
            tables: vec![unwritten; pack.tables.len()],
            ..ProgramCode::default()
        },
        rule: SelectedRuleOrdinal(0),
        constants: HashMap::new(),
        indexes: HashMap::new(),
        patterns: HashMap::new(),
        reached: Vec::new(),
        written: Count(0),
        entries: Vec::new(),
        tables: vec![TableProgress::Unreached; pack.tables.len()],
        need_places: Vec::new(),
        need_records: Vec::new(),
        need_offsets: Vec::new(),
        definition_needs: HashMap::new(),
        sizes: HashSet::new(),
        memos: WordSlot(0),
        unit: UnitPoints::default(),
        registers: pack.registers,
    }
}

/// Writes one rule: its own program as one unit (each branch's need, then its steps), then every unit it reached.
fn rule_written(encoding: &mut Encoding<'_>, rule: &LoweredRule) -> Result<(), LoweringError> {
    encoding.rule = rule.rule;
    begin_unit(
        encoding,
        UnitKey::Rule(rule.node),
        MeasuredForm::Call,
        ShapeChoice::Decided,
    )?;
    let mut branches = Vec::with_capacity(rule.branches.len());
    for branch in &rule.branches {
        let need = need_site(encoding, &branch.emptiness)?;
        let entry = program_entry(encoding, &branch.program, ProgramRole::Branch)?;
        branches.push(BranchCode { need, entry });
    }
    end_unit(encoding)?;
    encoding.code.rules.push(RuleCode {
        rule: rule.rule,
        branches: branches.into(),
        columns: rule.columns.clone(),
    });
    reached_written(encoding)?;
    counted(encoding)
}

/// Holds every count of the pack so far to its width: the refusal of the rule being written when one is past it.
fn counted(encoding: &Encoding<'_>) -> Result<(), LoweringError> {
    let _words = slot_of(encoding, Count(encoding.code.words.len()), PastWidth::Words)?;
    let pool = slot_of(encoding, Count(encoding.code.pool.len()), PastWidth::Words)?;
    // A place of the pool is never the greatest slot: that one says "none".
    if pool == NO_SLOT {
        return Err(past(encoding, PastWidth::Words));
    }
    Ok(())
}

/// The lowered pack as code: operands decided, sites and indexes numbered, each rule's own program then what it reached.
///
/// # Errors
///
/// The rule being written when a count does not fit its width (`Budget`, naming what was counted), and when a step or a stage
/// stands where the contract does not let it (`Unlowerable`, the engine's own fault: the lowering proves a pack before it
/// answers it).
pub(crate) fn encoded(pack: &LoweredPack) -> Result<ProgramCode, LoweringError> {
    let mut encoding = started(pack);
    for rule in &pack.rules {
        rule_written(&mut encoding, rule)?;
    }
    entries_written(&mut encoding)?;
    let needs = assembled(&encoding)?;
    let counts = StateCounts {
        registers: WordSlot(encoding.registers.0),
        memos: encoding.memos,
        indexes: slot_of(
            &encoding,
            Count(encoding.code.indexes.len()),
            PastWidth::Sites,
        )?,
        tables: slot_of(
            &encoding,
            Count(encoding.code.tables.len()),
            PastWidth::Sites,
        )?,
        needs: slot_of(
            &encoding,
            Count(encoding.need_places.len()),
            PastWidth::Sites,
        )?,
        relations: slot_of(
            &encoding,
            Count(encoding.code.relations.len()),
            PastWidth::Sites,
        )?,
        points: encoding.code.counts.points,
        units: slot_of(
            &encoding,
            Count(encoding.code.units.len()),
            PastWidth::Sites,
        )?,
    };
    let mut code = encoding.code;
    code.needs = needs;
    code.counts = counts;
    Ok(code)
}
