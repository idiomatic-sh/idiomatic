//! Rebind a program's supplied relations and step numbers into the reader's plan.
use super::{Bindings, Bound, Plan, Run, Table, Tables, finish};
use ryo_engine::domain::plan_lowering::shared::SharedSteps;
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep};
use std::{collections::HashMap, rc::Rc};

type Aliases = HashMap<String, String>;

/// A program's lowered steps depend on its host aliases. The values supplied
/// under those aliases and the frame are bound anew on every SDK call.
pub(super) struct PreviousPlan {
    inputs: Vec<(usize, Aliases)>,
    plan: Rc<Plan>,
    slots: Vec<Vec<usize>>,
}

#[derive(Default)]
pub(super) struct ReaderPlan {
    steps: SharedSteps,
    inputs: Vec<(usize, Aliases)>,
    hosts: HashMap<Rc<Table>, String>,
    outputs: Vec<(String, usize, String, Vec<String>)>,
    tests: HashMap<(bool, Vec<Vec<(bool, u32)>>, Vec<Vec<u32>>), usize>,
}

impl ReaderPlan {
    pub(super) fn add(&mut self, b: &mut Bound<'_>, run: &mut Run, program: usize, retain: bool) {
        let next = self.tests.len();
        b.test = *self
            .tests
            .entry((b.never, b.shape.clone(), b.words.clone()))
            .or_insert(next);
        let mut given = Aliases::with_capacity(b.hosted.len());
        for (name, table) in &b.hosted {
            let next = self.hosts.len();
            let alias = self.hosts.entry(Rc::clone(table)).or_insert_with(|| {
                // This internal spelling is outside the identifier grammar.
                let alias = format!("\0input/{next}");
                let fields: Vec<_> = table
                    .names
                    .iter()
                    .zip(table.tys.iter())
                    .map(|(n, t)| (n.as_str(), *t))
                    .collect();
                run.declare(&alias, &fields, finish::Supplier::Host);
                alias
            });
            given.insert(name.clone(), alias.clone());
        }
        if retain {
            self.inputs.push((program, given));
        } else {
            self.append(b, &given);
        }
    }

    fn append(&mut self, b: &mut Bound<'_>, given: &Aliases) {
        for node in &b.plan.nodes {
            let mut step = node.step().clone();
            if let FinishStep::Base(relation) = &mut step {
                if let Some(alias) = given.get(relation.0.as_ref()) {
                    relation.0 = alias.as_str().into();
                }
            }
            let slot = self.steps.append(step, &b.slots);
            b.slots.push(slot);
        }
        for (rule, output, subject, keys) in &b.plan.emits {
            self.outputs.push((
                rule.clone(),
                b.slots[*output],
                subject.clone(),
                keys.clone(),
            ));
        }
    }

    fn plan(&mut self) -> Rc<Plan> {
        let mut plan = finish::from_typed(std::sync::Arc::new(FinishPlan {
            steps: std::mem::take(&mut self.steps).finish(),
            rules: Vec::new(),
            unplanned: Vec::new(),
        }));
        plan.emits = std::mem::take(&mut self.outputs);
        Rc::new(plan)
    }

    pub(super) fn finish(
        mut self,
        previous: Option<&mut Option<PreviousPlan>>,
        bound: &mut Bindings<'_>,
    ) -> (Rc<Plan>, Tables, usize) {
        let plan = if let Some(last) = previous {
            let bindings = bound.entries.iter_mut().filter_map(|entry| entry.as_mut().ok());
            if let Some(hit) = last.as_ref().filter(|p| p.inputs == self.inputs) {
                for (binding, slots) in bindings.zip(&hit.slots) {
                    binding.slots.clone_from(slots);
                }
                hit.plan.clone()
            } else {
                let inputs = std::mem::take(&mut self.inputs);
                let slots = bindings.zip(&inputs).map(|(binding, (_, aliases))| {
                    self.append(binding, aliases);
                    binding.slots.clone()
                }).collect();
                let plan = self.plan();
                *last = Some(PreviousPlan { inputs, plan: plan.clone(), slots });
                plan
            }
        } else {
            self.plan()
        };
        let tables = self
            .hosts
            .into_iter()
            .map(|(table, name)| (name, table))
            .collect();
        (plan, tables, self.tests.len())
    }
}
