//! Rebind a program's supplied relations and step numbers into the reader's plan.
use super::{Bound, Plan, Run, Table, finish};
use ryo_engine::domain::plan_lowering::shared::SharedSteps;
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep};
use std::{collections::HashMap, rc::Rc};

pub type Aliases = HashMap<String, String>;

#[derive(Default)]
pub struct ReaderPlan {
    pub steps: SharedSteps,
    pub hosts: HashMap<Rc<Table>, String>,
    pub outputs: Vec<(String, usize, String, Vec<String>)>,
}

impl ReaderPlan {
    pub fn add(&mut self, b: &mut Bound<'_>, run: &mut Run) {
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
        for (relation, alias) in &b.layout {
            let name = ryo_relations::domain::relations::service::name(*relation).0;
            given.insert(name.to_owned(), alias.clone());
        }
        self.append(b, &given);
    }

    pub fn append(&mut self, b: &mut Bound<'_>, given: &Aliases) {
        let slots = Rc::make_mut(&mut b.slots);
        slots.reserve(b.plan.nodes.len());
        for node in &b.plan.nodes {
            let mut step = node.step().clone();
            if let FinishStep::Base(relation) = &mut step {
                if let Some(alias) = given.get(relation.0.as_ref()) {
                    relation.0 = alias.as_str().into();
                }
            }
            let slot = self.steps.append(step, slots);
            slots.push(slot);
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

    pub fn plan(&mut self) -> Rc<Plan> {
        let mut plan = finish::from_typed(std::sync::Arc::new(FinishPlan {
            steps: std::mem::take(&mut self.steps).finish(),
            rules: Vec::new(),
            unplanned: Vec::new(),
        }));
        plan.emits = std::mem::take(&mut self.outputs);
        Rc::new(plan)
    }

    pub fn finish(mut self) -> (Rc<Plan>, rustc_hash::FxHashMap<String, Rc<Table>>) {
        let plan = self.plan();
        let tables = self
            .hosts
            .into_iter()
            .map(|(table, name)| (name, table))
            .collect();
        (plan, tables)
    }
}
