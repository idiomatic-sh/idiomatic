//! Closed output shapes depend on the prepared plan and the declared provider
//! catalogue. Workers need neither source-word IDs nor table addresses to use one.
use super::{E, Exec, FxHashMap, Rc, Run, Schema, SelPlan, Ty};
use std::sync::Arc;

const UNBOUND: u32 = u32::MAX;

pub(crate) struct Shapes {
    slots: Vec<u32>,
    selects: Vec<Option<Box<SelPlan>>>,
    kernels: Vec<Option<Box<super::select_kernel::Prep>>>,
    unique: Vec<(Vec<String>, Vec<Ty>)>,
}

/// A visit can keep its bound select while it mutates other executor slots.
pub(super) enum Binding<'p, T> {
    Prepared(&'p T),
    Local(Rc<T>),
}

impl<T> Clone for Binding<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Prepared(value) => Self::Prepared(value),
            Self::Local(value) => Self::Local(value.clone()),
        }
    }
}

pub(super) type Selection<'p> = Binding<'p, SelPlan>;

impl<T> std::ops::Deref for Binding<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Prepared(p) => p,
            Self::Local(p) => p,
        }
    }
}

impl E {
    fn has_matcher(&self) -> bool {
        match self {
            Self::Re(..) => true,
            Self::List(xs) | Self::Call(_, xs) => xs.iter().any(Self::has_matcher),
            Self::Cmp(_, a, b) | Self::In(a, b, _) | Self::And(a, b) | Self::Or(a, b) => {
                a.has_matcher() || b.has_matcher()
            }
            Self::Not(x) => x.has_matcher(),
            Self::Col(..) | Self::Lit(_) | Self::Text(_) | Self::Num(_) => false,
        }
    }
}

impl Shapes {
    pub(crate) fn attach<'w>(self: &'w Arc<Self>, exec: &mut Exec<'w>) {
        exec.worker_shapes = Some(Local::new(self.clone()));
        exec.shared_sels = &self.selects;
        exec.shared_kernels = &self.kernels;
    }
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
        // These literal IDs precede the dictionary snapshot used by every child.
        // A select containing a regex binds locally to keep its search cache private.
        let selects: Vec<_> = self
            .sels
            .iter()
            .map(|slot| {
                let plan = slot.as_deref()?;
                if plan.conj.iter().any(E::has_matcher)
                    || plan.items.iter().any(|(_, e)| e.has_matcher())
                {
                    return None;
                }
                Some(Box::new(plan.clone()))
            })
            .collect();
        // The predicates use fixed literal IDs. Entry recipes can also contain fact
        // ordinals, so Settings preserves the completed fact prefix in every worker.
        let kernels = selects.iter().enumerate().map(|(node, binding)| {
            binding.as_ref()?;
            self.ksel_make(run, node).ok().map(Box::new)
        }).collect();
        Arc::new(Shapes {
            kernels,
            slots,
            unique,
            selects,
        })
    }
}
