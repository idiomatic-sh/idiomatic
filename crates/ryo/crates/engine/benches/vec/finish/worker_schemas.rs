//! Closed output shapes depend on the prepared plan and the declared provider
//! catalogue. Workers need neither source-word IDs nor table addresses to use one.
use super::{Exec, FxHashMap, Rc, Run, Schema, Ty};
use std::sync::Arc;

const UNBOUND: u32 = u32::MAX;

pub(crate) struct Shapes {
    slots: Vec<u32>,
    unique: Vec<(Vec<String>, Vec<Ty>)>,
}

pub(crate) struct Local {
    shared: Arc<Shapes>,
    shapes: Vec<Option<Rc<Schema>>>,
}

impl Local {
    pub(crate) fn new(shared: Arc<Shapes>) -> Self {
        Self {
            shapes: vec![None; shared.unique.len()],
            shared,
        }
    }

    pub(super) fn get(&mut self, node: usize) -> Option<Rc<Schema>> {
        let id = self.shared.slots[node];
        if id == UNBOUND {
            return None;
        }
        let (names, types) = &self.shared.unique[id as usize];
        Some(
            self.shapes[id as usize]
                .get_or_insert_with(|| {
                    Rc::new(Schema {
                        names: Rc::new(names.clone()),
                        tys: Rc::new(types.clone()),
                    })
                })
                .clone(),
        )
    }
}

impl Exec<'_> {
    pub(crate) fn freeze_shapes(&mut self, run: &mut Run, emits: &[usize]) -> Arc<Shapes> {
        let t = super::super::thread_cpu();
        for &emit in emits {
            // A rule whose schema cannot be bound can still be screened out.
            // Leave that slot unbound so the actual visit retains its error behavior.
            let _ = self.schema(run, self.plan.emits[emit].1);
        }
        let mut ids = FxHashMap::default();
        let mut canonical = Vec::new();
        let mut unique = Vec::new();
        let mut slots = Vec::with_capacity(self.schemas.len());
        for schema in &self.schemas {
            let Some(sc) = schema else {
                slots.push(UNBOUND);
                continue;
            };
            let signature = (sc.names.as_slice(), sc.tys.as_slice());
            let id = *ids.entry(signature).or_insert_with(|| {
                let id = u32::try_from(unique.len()).expect("column shape ID");
                unique.push(((*sc.names).clone(), (*sc.tys).clone()));
                canonical.push(sc.clone());
                id
            });
            slots.push(id);
        }
        drop(ids);
        let mut bound = 0;
        for (&id, slot) in slots.iter().zip(&mut self.schemas) {
            if id != UNBOUND {
                *slot = Some(canonical[id as usize].clone());
                bound += 1;
            }
        }
        println!(
            "worker {} schema slots, {} distinct shapes CPU {:.6} s",
            bound,
            unique.len(),
            super::super::thread_cpu() - t
        );
        Arc::new(Shapes { slots, unique })
    }
}
