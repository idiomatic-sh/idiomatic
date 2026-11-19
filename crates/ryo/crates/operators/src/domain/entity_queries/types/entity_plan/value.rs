//! An immutable prepared entity program, with counts for file-local positions and physical exports.
use super::super::{
    AncestorDistance, EntityAttribute, EntityColumn, EntityColumnCount, EntityExport,
    EntityExportId, EntityLiteralCode, EntityOperand, EntityQueryOp, EntitySources, EntityStage,
};
use ryo_values::domain::{
    plan_parts::types::{LiteralText, Side, StepId},
    rows::types::RowCount,
};
use std::{
    borrow::Borrow,
    collections::{HashMap, HashSet},
};
/// A literal the plan may look up by the bytes of a borrowed file spelling.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct EntitySpelling(LiteralText);
impl Borrow<str> for EntitySpelling {
    fn borrow(&self) -> &str {
        &self.0.0
    }
}
/// Immutable stages shared by all files in the prepared physical plan.
#[derive(Clone, Debug, Default)]
pub(crate) struct QueryPlan {
    pub(in super::super::super) partitions: Option<std::sync::Arc<crate::domain::row_predicates::types::Program>>,
    pub(in super::super::super) stages: Vec<EntityStage>,
    pub(in super::super::super) names: HashMap<EntitySpelling, EntityLiteralCode>,
    pub(in super::super::super) readers: Vec<RowCount>,
    pub(in super::super::super) ancestor_columns: EntityColumnCount,
    pub(in super::super::super) field_slots: EntityColumnCount,
    pub(in super::super::super) max_width: EntityColumnCount,
    pub(crate) sources: Vec<EntitySources>,
    exports: Vec<Option<StepId>>,
}
impl QueryPlan {
    pub(crate) fn from_stages(mut stages: Vec<EntityStage>, ports: Vec<EntityExport>) -> Self {
        if ports.is_empty() {
            return Self::default();
        }
        let mut exports = vec![
            None;
            ports
                .iter()
                .map(|export| export.0.0 as usize)
                .max()
                .unwrap()
                + 1
        ];
        let mut names = HashMap::new();
        let mut readers = vec![0; stages.len()];
        let mut depths = vec![0; stages.len()];
        let mut sources = vec![0; stages.len()];
        let mut ancestry = HashSet::new();
        let mut field_slots = 0;
        let mut max_width = 0;
        for EntityExport(physical, step) in ports {
            let stage = usize::from(step);
            debug_assert_eq!(stages[stage].physical, Some(physical));
            exports[physical.0 as usize] = Some(step);
            readers[stage] += 1;
        }
        for i in 0..stages.len() {
            let (previous, current) = stages.split_at_mut(i);
            let stage = &mut current[0];
            for &p in &stage.operands {
                debug_assert!(usize::from(p) < i);
                readers[usize::from(p)] += 1;
            }
            sources[i] = match stage.expr {
                EntityQueryOp::Syntax => 1,
                EntityQueryOp::Edge => 2,
                _ => stage
                    .operands
                    .iter()
                    .fold(0, |mask, &p| mask | sources[usize::from(p)]),
            };
            depths[i] = stage
                .operands
                .iter()
                .map(|&p| depths[usize::from(p)] + 1)
                .max()
                .unwrap_or(0);
            if depths[i] > 256 {
                return Self::default();
            }
            max_width = max_width.max(stage.fields.len());
            for &EntityColumn(field, AncestorDistance(depth)) in &stage.fields {
                if depth > 128 {
                    return Self::default();
                }
                if depth > 0 && field != EntityAttribute::File {
                    ancestry.insert((field, depth));
                }
                field_slots = field_slots.max(7 * depth + field as usize + 1);
            }
            match &mut stage.expr {
                EntityQueryOp::Filter { tests } => {
                    for test in tests {
                        if let EntityOperand::Text(text, code) = &mut test.operand {
                            let next = EntityLiteralCode(names.len() as u32);
                            *code = *names.entry(EntitySpelling(text.clone())).or_insert(next);
                        }
                    }
                }
                EntityQueryOp::Join { side, depth, l, r } => {
                    let a = &previous[usize::from(stage.operands[0])].fields;
                    let b = &previous[usize::from(stage.operands[1])].fields;
                    let ls = if *side == Side::Right { depth.0 } else { 0 };
                    let rs = if *side == Side::Left { depth.0 } else { 0 };
                    let mut keep = Vec::new();
                    for (&x, &y) in l.iter().zip(r.iter()) {
                        let EntityColumn(lk, ld) = a[usize::from(x)];
                        let EntityColumn(rk, rd) = b[usize::from(y)];
                        if lk != rk || (lk != EntityAttribute::File && ld.0 + ls != rd.0 + rs) {
                            keep.push((x, y));
                        }
                    }
                    (*l, *r) = keep.into_iter().unzip();
                }
                _ => {}
            }
        }
        Self {
            partitions: super::super::super::service::entity_partitions(&stages),
            stages,
            names,
            readers: readers.into_iter().map(RowCount).collect(),
            exports,
            ancestor_columns: EntityColumnCount(ancestry.len()),
            field_slots: EntityColumnCount(field_slots),
            max_width: EntityColumnCount(max_width),
            sources: sources.into_iter().map(EntitySources).collect(),
        }
    }
    pub(crate) fn exported(&self, physical: EntityExportId) -> Option<StepId> {
        self.exports.get(physical.0 as usize).copied().flatten()
    }
}
