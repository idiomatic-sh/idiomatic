//! Rebind a program's supplied relations and step numbers into the reader's plan.
use super::{Bound, Plan, Run, Table, Tables, finish};
use ryo_engine::domain::plan_lowering::shared::SharedSteps;
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep};
use std::{collections::HashMap, rc::Rc};

#[derive(Default)]
pub(super) struct ReaderPlan {
    steps: SharedSteps,
    hosts: HashMap<Rc<Table>, String>,
    outputs: Vec<(String, usize, String, Vec<String>)>,
    tests: HashMap<(bool, Vec<Vec<(bool, u32)>>, Vec<Vec<u32>>), usize>,
}

impl ReaderPlan {
    pub(super) fn add(&mut self, b: &mut Bound<'_>, run: &mut Run) {
        let next = self.tests.len();
        b.test = *self
            .tests
            .entry((b.never, b.shape.clone(), b.words.clone()))
            .or_insert(next);
        let mut given = HashMap::new();
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
            given.insert(name.as_str(), alias.as_str().to_owned());
        }
        for node in &b.plan.nodes {
            let mut step = node.step().expect("prepared typed program").clone();
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

    pub(super) fn finish(self) -> (Plan, Tables, usize) {
        let mut plan = finish::from_typed(Rc::new(FinishPlan {
            steps: self.steps.finish(),
            rules: Vec::new(),
            unplanned: Vec::new(),
        }));
        plan.emits = self.outputs;
        let tables = self
            .hosts
            .into_iter()
            .map(|(table, name)| (name, table))
            .collect();
        (plan, tables, self.tests.len())
    }
}
