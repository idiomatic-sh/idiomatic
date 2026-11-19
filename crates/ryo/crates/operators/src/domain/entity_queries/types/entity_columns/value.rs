//! Borrowless columns and lazy parent fields for one physical file generation.
use super::super::super::service::{shape, spelling};
use super::super::entity_plan::value::QueryPlan;
use super::super::{
    AncestorDistance, EntityAttribute, EntityColumn, EntityLiteralCode, EntityPosition,
};
use crate::domain::table_texts::types::{TextView, Texts};
use ryo_relations::domain::{
    relations::types::{Bits, CellKind, ColumnsView, FactColumn, FileId, NodeId},
    terms::types::Symbol,
};
use ryo_values::domain::rows::types::RowCount;
use std::sync::{Arc, OnceLock};
const ABSENT: EntityPosition = EntityPosition(u32::MAX);
const FILE_LIMIT: usize = 16 * 1024 * 1024;
/// An ordered file-local bag of entity positions.
#[derive(Clone, Debug)]
pub(in super::super::super) struct EntityPositions(Arc<[EntityPosition]>);
impl From<Vec<EntityPosition>> for EntityPositions {
    fn from(rows: Vec<EntityPosition>) -> Self {
        Self(rows.into())
    }
}
impl std::ops::Deref for EntityPositions {
    type Target = [EntityPosition];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy)]
pub(in super::super::super) enum EntityColumnWords<'a> {
    Constant(Bits),
    Identity(NodeId),
    Column(&'a FactColumn),
}
impl EntityColumnWords<'_> {
    #[inline]
    pub(in super::super::super) fn get(self, p: EntityPosition) -> Bits {
        match self {
            Self::Constant(v) => v,
            Self::Identity(file) => Bits(file.0 | u64::from(p.0)),
            Self::Column(words) => Bits(words.0[p.0 as usize]),
        }
    }
}

#[derive(Debug)]
pub(in super::super::super) struct EntityFileColumns {
    identity: NodeId,
    parents: Vec<EntityPosition>,
    data: [FactColumn; 5],
    source: [EntityPositions; 2],
    codes: rustc_hash::FxHashMap<Symbol, Option<EntityLiteralCode>>,
    // Spike (ryo-fmx3): each field's literal code by position, made on first use (u32::MAX: none).
    coded: Vec<OnceLock<Vec<u32>>>,
    no_spelling: Option<EntityLiteralCode>,
    ancestors: Vec<OnceLock<FactColumn>>,
}
impl EntityFileColumns {
    pub(in super::super::super) fn open(
        plan: &QueryPlan,
        syntax: ColumnsView<'_>,
        edge: ColumnsView<'_>,
        texts: &Texts<'_>,
    ) -> Option<Self> {
        use CellKind::{File as FileKind, Node, Number, Span, Symbol as SymbolKind};
        let nr = shape(syntax, &[Node, SymbolKind, SymbolKind, FileKind, Span])?.0;
        let ne = shape(edge, &[Node, Node, Number, SymbolKind, FileKind])?.0;
        let saved_file = syntax.columns[3]
            .0
            .first()
            .or_else(|| edge.columns[4].0.first())
            .copied()
            .unwrap_or(0);
        let Ok(file) = u32::try_from(saved_file) else {
            return None;
        };
        let identity = NodeId(u64::from(file) << 32);
        let size = syntax.columns[0]
            .0
            .iter()
            .chain(&edge.columns[0].0)
            .chain(&edge.columns[1].0)
            .map(|&word| (word as u32) as usize + 1)
            .max()
            .unwrap_or(0);
        let per_entity = 56usize.checked_add(plan.ancestor_columns.0.checked_mul(8)?)?;
        let budget = size
            .checked_mul(per_entity)?
            .checked_add(nr.checked_add(ne)?.checked_mul(4)?)?;
        if budget > FILE_LIMIT || size.checked_mul(plan.max_width.0)? > 2 * 1024 * 1024 {
            return None;
        }
        let mut data: [FactColumn; 5] = std::array::from_fn(|_| FactColumn(vec![0; size]));
        let mut parents = vec![ABSENT; size];
        let mut observed = vec![false; size];
        let mut si = Vec::with_capacity(nr);
        let mut ei = Vec::with_capacity(ne);
        let mut codes = rustc_hash::FxHashMap::default();
        let no_spelling = plan.names.get(TextView::default().0).copied();
        let mut remember = |raw| {
            let Ok(raw) = u32::try_from(raw) else {
                return None;
            };
            let symbol = Symbol(raw);
            if let std::collections::hash_map::Entry::Vacant(place) = codes.entry(symbol) {
                let spelling = spelling(texts, symbol)?;
                place.insert(plan.names.get(spelling.0).copied());
            }
            Some(u64::from(raw))
        };
        for i in 0..nr {
            let id = syntax.columns[0].0[i];
            if id >> 32 != saved_file || syntax.columns[3].0[i] != saved_file {
                return None;
            }
            let p = id as u32;
            let j = p as usize;
            if std::mem::replace(&mut observed[j], true) {
                return None;
            }
            si.push(EntityPosition(p));
            data[0].0[j] = remember(syntax.columns[1].0[i])?;
            data[1].0[j] = remember(syntax.columns[2].0[i])?;
            data[2].0[j] = syntax.columns[4].0[i];
        }
        for i in 0..ne {
            let parent = edge.columns[0].0[i];
            let id = edge.columns[1].0[i];
            let p = id as u32;
            if id >> 32 != saved_file
                || parent >> 32 != saved_file
                || edge.columns[4].0[i] != saved_file
                || parent as u32 == ABSENT.0
            {
                return None;
            }
            if parents[p as usize] != ABSENT {
                return None;
            }
            parents[p as usize] = EntityPosition(parent as u32);
            ei.push(EntityPosition(p));
            data[3].0[p as usize] = edge.columns[2].0[i];
            data[4].0[p as usize] = remember(edge.columns[3].0[i])?;
        }
        Some(Self {
            identity,
            parents,
            data,
            source: [si.into(), ei.into()],
            codes,
            no_spelling,
            ancestors: (0..plan.field_slots.0).map(|_| OnceLock::new()).collect(),
            coded: (0..plan.field_slots.0.max(7)).map(|_| OnceLock::new()).collect(),
        })
    }
    pub(in super::super::super) fn len(&self) -> RowCount {
        RowCount(self.parents.len())
    }
    pub(in super::super::super) fn source(
        &self,
        which: ryo_values::domain::plan_parts::types::Side,
    ) -> EntityPositions {
        self.source[match which {
            ryo_values::domain::plan_parts::types::Side::Left => 0,
            ryo_values::domain::plan_parts::types::Side::Right => 1,
        }]
        .clone()
    }
    pub(in super::super::super) fn code(&self, raw: Bits) -> Option<EntityLiteralCode> {
        self.codes
            .get(&Symbol(raw.0 as u32))
            .copied()
            .unwrap_or(self.no_spelling)
    }
    /// Spike (ryo-fmx3): the literal code of `column` at each position, `u32::MAX` where it has none: what `code` answers
    /// for the column's word there, read once per file and field.
    pub(in super::super::super) fn codes_of(&self, column: EntityColumn) -> &[u32] {
        let EntityColumn(field, AncestorDistance(depth)) = column;
        let slot = 7 * depth + field as usize;
        let Some(place) = self.coded.get(slot) else {
            return &[];
        };
        place.get_or_init(|| {
            let words = self.field(column);
            (0..self.len().0)
                .map(|p| {
                    self.code(words.get(EntityPosition(p as u32)))
                        .map_or(u32::MAX, |code| code.0)
                })
                .collect()
        })
    }
    pub(in super::super::super) fn parent(
        &self,
        mut current: EntityPosition,
        depth: AncestorDistance,
    ) -> Option<EntityPosition> {
        for _ in 0..depth.0 {
            current = *self.parents.get(current.0 as usize)?;
            if current == ABSENT {
                return None;
            }
        }
        Some(current)
    }
    pub(in super::super::super) fn field(
        &self,
        EntityColumn(field, AncestorDistance(depth)): EntityColumn,
    ) -> EntityColumnWords<'_> {
        let direct = match field {
            EntityAttribute::File => {
                return EntityColumnWords::Constant(Bits(self.identity.0 >> 32));
            }
            EntityAttribute::Id => EntityColumnWords::Identity(self.identity),
            EntityAttribute::Kind => EntityColumnWords::Column(&self.data[0]),
            EntityAttribute::Text => EntityColumnWords::Column(&self.data[1]),
            EntityAttribute::Span => EntityColumnWords::Column(&self.data[2]),
            EntityAttribute::Ordinal => EntityColumnWords::Column(&self.data[3]),
            EntityAttribute::Role => EntityColumnWords::Column(&self.data[4]),
        };
        if depth == 0 {
            return direct;
        }
        let column = self.ancestors[7 * depth + field as usize].get_or_init(|| {
            FactColumn(
                (0..self.len().0)
                    .map(|j| {
                        self.parent(EntityPosition(j as u32), AncestorDistance(depth))
                            .map_or(0, |p| direct.get(p).0)
                    })
                    .collect(),
            )
        });
        EntityColumnWords::Column(column)
    }
}
