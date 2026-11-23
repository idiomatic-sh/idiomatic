//! A lowered literal pattern has one compilation per request. A worker clones
//! its matcher on first use to give the automaton a private search cache.
use super::{Plan, R, Run, glob_pattern};
use regex::bytes::Regex;
use rustc_hash::FxHashMap;
use ryo_values::domain::finish_plans::{
    expressions::types::{PlanConstant, PlanExpression},
    types::{FinishStep, RowCondition, SelectedColumns},
};
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct Patterns(FxHashMap<String, R<Regex>>);

impl Patterns {
    pub(crate) fn for_workers(plan: &Plan, run: &mut Run) -> Arc<Self> {
        let mut patterns = Self::default();
        for (name, compiled) in &run.regexes {
            let _ = patterns.0.insert(name.clone(), Ok(compiled.as_ref().clone()));
        }
        for node in &plan.nodes {
            match node.step() {
                FinishStep::Capture(s) => patterns.add(s.pattern.0.to_string()),
                FinishStep::SourceMatches(s) => patterns.add(s.pattern.0.to_string()),
                FinishStep::Replace(s) => patterns.add(s.pattern.0.to_string()),
                FinishStep::Select(s) => {
                    if let RowCondition::Tested(expr) = &s.condition {
                        patterns.collect(expr);
                    }
                    if let SelectedColumns::Items(items) = &s.columns {
                        for item in items {
                            patterns.collect(&item.value);
                        }
                    }
                }
                _ => {}
            }
        }
        let shared = Arc::new(patterns);
        run.worker_patterns = Some(shared.clone());
        shared
    }
    pub(super) fn get(&self, pattern: &str) -> Option<R<Regex>> {
        self.0.get(pattern).cloned()
    }
    fn add(&mut self, pattern: String) {
        self.0.entry(pattern).or_insert_with_key(|p| compile(p));
    }
    fn collect(&mut self, e: &PlanExpression) {
        use PlanExpression::*;
        match e {
            Call(c) => {
                if let [_, value] = c.arguments.as_slice() {
                    match c.function.0.as_ref() {
                        "matches" => self.literal(value, false),
                        "glob_any" => self.literal(value, true),
                        _ => {}
                    }
                }
                for arg in &c.arguments {
                    self.collect(arg);
                }
            }
            Compare(c) => {
                self.collect(&c.left);
                self.collect(&c.right);
            }
            In(c) | NotIn(c) | And(c) | Or(c) => {
                self.collect(&c.left);
                self.collect(&c.right);
            }
            List(items) => {
                for item in items {
                    self.collect(item);
                }
            }
            Not(child) => self.collect(child),
            _ => {}
        }
    }
    fn literal(&mut self, e: &PlanExpression, glob: bool) {
        match e {
            PlanExpression::Literal(PlanConstant::Text(t)) => {
                self.add(if glob {
                    glob_pattern(&t.0)
                } else {
                    t.0.to_string()
                });
            }
            PlanExpression::List(items) if glob => {
                for item in items {
                    self.literal(item, true);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn compile(pattern: &str) -> R<Regex> {
    Regex::new(pattern).map_err(|e| format!("regex {pattern}: {e}"))
}
