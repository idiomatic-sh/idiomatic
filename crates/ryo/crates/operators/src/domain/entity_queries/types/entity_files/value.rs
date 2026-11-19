//! Ordered lazy entity stages for one open file and prepared plan.
use super::super::super::service::kind;
use super::super::entity_columns::value::{EntityColumnWords, EntityFileColumns, EntityPositions};
use super::super::entity_plan::value::QueryPlan;
use super::super::{
    AncestorDistance, EntityAttribute, EntityColumn, EntityOperand, EntityOwnership,
    EntityPosition, EntityQueryOp, EntitySources,
};
use crate::domain::table_texts::types::Texts;
use ryo_relations::domain::relations::{
    service::{from_bits, kinds},
    types::{Bits, ColumnsView, FactColumn, Relation},
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, RepeatedRows, Side, StepId, Test},
    rows::types::{RowCount, Table},
};
use std::{
    collections::{HashSet, VecDeque},
    sync::{Arc, Weak},
};
const POSITION_ALLOWANCE: usize = 2 * 1024 * 1024;
const EXPORT_CELLS: usize = 2 * 1024 * 1024;
#[derive(Debug)]
pub(crate) struct FileQueries {
    subscriptions: super::super::entity_subscriptions::value::EntitySubscriptions,
    owner: Weak<QueryPlan>,
    graph: EntityFileColumns,
    cached: Vec<Option<EntityPositions>>,
    eviction: VecDeque<StepId>,
    held: RowCount,
    pub(crate) sources: EntitySources,
}
impl FileQueries {
    pub(crate) fn open(
        plan: &Arc<QueryPlan>,
        syntax: Option<ColumnsView<'_>>,
        edges: Option<ColumnsView<'_>>,
        texts: &Texts<'_>,
    ) -> Option<Self> {
        let sources = u8::from(syntax.is_some()) | u8::from(edges.is_some()) << 1;
        let empty: [FactColumn; 5] = std::array::from_fn(|_| FactColumn(Vec::new()));
        let syntax = syntax.unwrap_or(ColumnsView {
            kinds: kinds(Relation::SemanticSyntax),
            columns: &empty,
        });
        let edges = edges.unwrap_or(ColumnsView {
            kinds: kinds(Relation::SemanticChild),
            columns: &empty,
        });
        Some(Self {
            subscriptions: Default::default(),
            owner: Arc::downgrade(plan),
            graph: EntityFileColumns::open(plan, syntax, edges, texts)?,
            cached: vec![None; plan.stages.len()],
            eviction: VecDeque::new(),
            held: RowCount(0),
            sources: EntitySources(sources),
        })
    }
    pub(crate) fn belongs_to(&self, plan: &Arc<QueryPlan>) -> EntityOwnership {
        if self.owner.ptr_eq(&Arc::downgrade(plan)) {
            EntityOwnership::Current
        } else {
            EntityOwnership::Foreign
        }
    }
    fn keep(&mut self, index: StepId, answer: &EntityPositions) {
        if answer.len() > POSITION_ALLOWANCE {
            return;
        }
        while self.held.0 + answer.len() > POSITION_ALLOWANCE {
            let Some(old) = self.eviction.pop_front() else {
                break;
            };
            if let Some(old) = self.cached[usize::from(old)].take() {
                self.held.0 -= old.len();
            }
        }
        self.held.0 += answer.len();
        self.cached[usize::from(index)] = Some(answer.clone());
        self.eviction.push_back(index);
    }
    fn distinct(&self, positions: &[EntityPosition], fields: &[EntityColumn]) -> EntityPositions {
        if fields.contains(&EntityColumn(EntityAttribute::Id, AncestorDistance(0))) {
            let mut seen = vec![false; self.graph.len().0];
            return positions
                .iter()
                .copied()
                .filter(|&p| !std::mem::replace(&mut seen[p.0 as usize], true))
                .collect::<Vec<_>>()
                .into();
        }
        let columns: Vec<_> = fields.iter().map(|&f| self.graph.field(f)).collect();
        macro_rules! fixed {
            ($n:expr) => {{
                let mut seen = HashSet::<[Bits; $n]>::new();
                positions
                    .iter()
                    .copied()
                    .filter(|&p| seen.insert(std::array::from_fn(|i| columns[i].get(p))))
                    .collect::<Vec<_>>()
                    .into()
            }};
        }
        match columns.len() {
            0 => fixed!(0),
            1 => fixed!(1),
            2 => fixed!(2),
            3 => fixed!(3),
            4 => fixed!(4),
            _ => {
                let mut seen = HashSet::new();
                positions
                    .iter()
                    .copied()
                    .filter(|&p| seen.insert(columns.iter().map(|c| c.get(p)).collect::<Vec<_>>()))
                    .collect::<Vec<_>>()
                    .into()
            }
        }
    }
    fn positions(&mut self, plan: &QueryPlan, step: StepId) -> EntityPositions {
        let i = usize::from(step);
        if let Some(answer) = &self.cached[i] {
            return answer.clone();
        }
        let stage = &plan.stages[i];
        let inputs: Vec<_> = stage
            .operands
            .iter()
            .map(|&p| self.positions(plan, p))
            .collect();
        let layout = |side| &plan.stages[usize::from(stage.operands[side])].fields;
        let field = |side, c: ColumnIndex| self.graph.field(layout(side)[usize::from(c)]);
        let answer: EntityPositions = match &stage.expr {
            EntityQueryOp::Syntax => self.graph.source(Side::Left),
            EntityQueryOp::Edge => self.graph.source(Side::Right),
            EntityQueryOp::Project { distinct } => {
                if *distinct == RepeatedRows::Dropped
                    && !stage
                        .fields
                        .contains(&EntityColumn(EntityAttribute::Id, AncestorDistance(0)))
                {
                    self.distinct(&inputs[0], &stage.fields)
                } else {
                    inputs[0].clone()
                }
            }
            EntityQueryOp::Filter { tests } => {
                let selected = plan.partitions.as_ref().and_then(|program|
                    self.subscriptions.selected(program, step, &inputs[0], layout(0), &self.graph));
                if tests.len() == 1 && let Some(positions) = selected {
                    positions
                } else {
                let candidates = selected.as_ref().unwrap_or(&inputs[0]);
                let predicates: Vec<_> = tests
                    .iter()
                    .map(|test| {
                        let other = match &test.operand {
                            EntityOperand::Column(c) => Some(field(0, *c)),
                            EntityOperand::Text(..) => None,
                        };
                        (field(0, test.column), other, test)
                    })
                    .collect();
                candidates.iter()
                    .copied()
                    .filter(|&p| {
                        predicates.iter().all(|(left, right, test)| {
                            let w = left.get(p);
                            let matched = match (&test.operand, right) {
                                (EntityOperand::Text(_, id), _) => self.graph.code(w) == Some(*id),
                                (_, Some(r)) => w == r.get(p),
                                _ => unreachable!(),
                            };
                            matched == (test.test == Test::Equal)
                        })
                    })
                    .collect::<Vec<_>>()
                    .into()
            }
            }
            EntityQueryOp::Union => {
                let mut rows = Vec::with_capacity(inputs.iter().map(|p| p.len()).sum());
                for v in &inputs {
                    rows.extend_from_slice(v);
                }
                self.distinct(&rows, &stage.fields)
            }
            EntityQueryOp::Anti { l, r } => {
                let left: Vec<_> = l.iter().map(|&c| field(0, c)).collect();
                let right: Vec<_> = r.iter().map(|&c| field(1, c)).collect();
                macro_rules! fixed {
                    ($n:expr) => {{
                        let keys: HashSet<[Bits; $n]> = inputs[1]
                            .iter()
                            .map(|&p| std::array::from_fn(|i| right[i].get(p)))
                            .collect();
                        inputs[0]
                            .iter()
                            .copied()
                            .filter(|&p| !keys.contains(&std::array::from_fn(|i| left[i].get(p))))
                            .collect::<Vec<_>>()
                            .into()
                    }};
                }
                match l.len() {
                    1 => fixed!(1),
                    2 => fixed!(2),
                    3 => fixed!(3),
                    4 => fixed!(4),
                    _ => {
                        let key = |p, cs: &[EntityColumnWords<'_>]| {
                            cs.iter().map(|c| c.get(p)).collect::<Vec<_>>()
                        };
                        let keys: HashSet<_> = inputs[1].iter().map(|&p| key(p, &right)).collect();
                        inputs[0]
                            .iter()
                            .copied()
                            .filter(|&p| !keys.contains(&key(p, &left)))
                            .collect::<Vec<_>>()
                            .into()
                    }
                }
            }
            EntityQueryOp::Join { side, depth, l, r } => {
                let drive = match side {
                    Side::Left => 0,
                    Side::Right => 1,
                };
                let target = 1 - drive;
                let mut location = vec![u32::MAX; self.graph.len().0];
                for (pos, &entity) in inputs[target].iter().enumerate() {
                    location[entity.0 as usize] = pos as u32;
                }
                let checks: Vec<_> = l
                    .iter()
                    .zip(r)
                    .map(|(&a, &b)| (field(0, a), field(1, b)))
                    .collect();
                let mut found = Vec::new();
                for &entity in inputs[drive].iter() {
                    let Some(bound) = self.graph.parent(entity, *depth) else {
                        continue;
                    };
                    let order = location[bound.0 as usize];
                    if order == u32::MAX {
                        continue;
                    }
                    let (a, b) = if drive == 0 {
                        (entity, bound)
                    } else {
                        (bound, entity)
                    };
                    if checks.iter().all(|(x, y)| x.get(a) == y.get(b)) {
                        found.push((order, entity));
                    }
                }
                if drive == 1 {
                    found.sort_by_key(|x| x.0);
                }
                found.into_iter().map(|x| x.1).collect::<Vec<_>>().into()
            }
        };
        if plan.readers[i].0 > 1 {
            self.keep(step, &answer);
        }
        answer
    }
    pub(crate) fn answer(&mut self, plan: &QueryPlan, stage: StepId) -> Option<Table> {
        let rows = self.positions(plan, stage);
        let layout = &plan.stages[usize::from(stage)].fields;
        let count = rows.len().checked_mul(layout.len())?;
        if count > EXPORT_CELLS {
            return None;
        }
        let kinds: Vec<_> = layout.iter().map(|c| kind(c.0)).collect();
        let fields: Vec<_> = layout.iter().map(|&f| self.graph.field(f)).collect();
        let mut cells = Vec::with_capacity(count);
        for &p in rows.iter() {
            for (&k, c) in kinds.iter().zip(&fields) {
                cells.push(from_bits(k, c.get(p)));
            }
        }
        Some(Table {
            kinds,
            cells,
            count: RowCount(rows.len()),
        })
    }
}

/// File-specific demand and the immutable sources a host has already offered for those queries.
#[derive(Debug, Default)]
pub(crate) struct EntityFileSession {
    file: Option<FileQueries>,
    owner: Weak<QueryPlan>,
    requested: EntitySources,
    attempted: EntitySources,
}
impl EntityFileSession {
    /// Ask in the ordinary consumer's position. The first request records the immutable sources to acquire.
    pub(crate) fn answer(&mut self, plan: &Arc<QueryPlan>, stage: StepId) -> Option<Table> {
        let needs = plan.sources[usize::from(stage)].0;
        if let Some(file) = self.file.as_mut()
            && file.belongs_to(plan) == EntityOwnership::Current
            && file.sources.0 & needs == needs
        {
            return file.answer(plan, stage);
        }
        if !std::ptr::eq(self.owner.as_ptr(), Arc::as_ptr(plan)) {
            *self = Self::default();
            self.owner = Arc::downgrade(plan);
        }
        self.requested.0 |= needs & !self.attempted.0;
        None
    }
    /// Incorporate only previously requested, complete columns from the unchanged file.
    pub(crate) fn bind<'a>(
        &mut self,
        plan: &Arc<QueryPlan>,
        texts: &Texts<'_>,
        read: impl Fn(Relation) -> Option<ColumnsView<'a>>,
    ) {
        let requested = std::mem::take(&mut self.requested);
        if requested.0 == 0 || !std::ptr::eq(self.owner.as_ptr(), Arc::as_ptr(plan)) {
            return;
        }
        self.attempted.0 |= requested.0;
        let held = self.file.as_ref().map_or(0, |file| file.sources.0);
        let both = held | requested.0;
        let syntax = (both & 1 != 0)
            .then(|| read(Relation::SemanticSyntax))
            .flatten();
        let children = (both & 2 != 0)
            .then(|| read(Relation::SemanticChild))
            .flatten();
        let available = u8::from(syntax.is_some()) | (u8::from(children.is_some()) << 1);
        if available & !held != 0
            && available & held == held
            && let Some(file) = FileQueries::open(plan, syntax, children, texts)
        {
            self.file = Some(file);
        }
    }
}
