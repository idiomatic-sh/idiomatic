//! A missing row kind is immaterial only when every direct consumer excludes it.
use super::finish::Plan;
use ryo::languages::Relation;
use ryo_relations::domain::relations::service as catalog;
use ryo_relations::domain::relations::types::{Missing, RowKind};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct Coverage(HashMap<(Relation, RowKind), bool>);

impl Coverage {
    pub(crate) fn needed(&mut self, plan: &Plan, relation: Relation, missing: Missing) -> bool {
        let Missing::Kind(kind) = missing else {
            return true;
        };
        *self.0.entry((relation, kind)).or_insert_with(|| {
            let Some(column) = catalog::kind_column(relation) else {
                return true;
            };
            let column = catalog::column_name(column).0;
            let name = catalog::name(relation).0;
            plan.nodes.iter().enumerate().any(|(base, node)| {
                node.op == "base"
                    && node.j["rel"] == name
                    && (plan.emits.iter().any(|emit| emit.1 == base)
                        || plan
                            .nodes
                            .iter()
                            .filter(|n| n.inputs.contains(&base))
                            .any(|n| {
                                n.op != "select"
                                    || n.inputs.len() != 1
                                    || !excludes(&n.j["cond"], column, kind.0)
                            }))
            })
        })
    }
}

fn excludes(condition: &Value, column: &str, missing: &str) -> bool {
    let different = |field: &Value, literal: &Value| {
        field[0] == "col"
            && field[1] == column
            && literal[0] == "lit"
            && literal[1].as_str().is_some_and(|kind| kind != missing)
    };
    match condition[0].as_str() {
        Some("cmp") if condition[1] == "=" => {
            different(&condition[2], &condition[3]) || different(&condition[3], &condition[2])
        }
        Some("and") => {
            excludes(&condition[1], column, missing) || excludes(&condition[2], column, missing)
        }
        Some("or") => {
            excludes(&condition[1], column, missing) && excludes(&condition[2], column, missing)
        }
        _ => false,
    }
}
