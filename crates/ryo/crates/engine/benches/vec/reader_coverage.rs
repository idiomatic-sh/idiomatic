//! A missing row kind is immaterial only when every direct consumer excludes it.
use super::finish::{Plan, PNode};
use ryo_values::domain::finish_plans::{expressions::types::*, types::*};
use ryo::languages::Relation;
use ryo_relations::domain::relations::service as catalog;
use ryo_relations::domain::relations::types::{Missing, RowKind};
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
                    && node.base() == Some(name)
                    && (plan.emits.iter().any(|emit| emit.1 == base)
                        || plan
                            .nodes
                            .iter()
                            .filter(|n| n.inputs.contains(&base))
                            .any(|n| {
                                n.op != "select"
                                    || n.inputs.len() != 1
                                    || !excludes_kind(n, column, kind.0)
                            }))
            })
        })
    }
}

fn excludes_kind(node: &PNode, column: &str, kind: &str) -> bool {
    if let FinishStep::Select(select) = node.step() {
        matches!(&select.condition, RowCondition::Tested(e) if typed_excludes(e, column, kind))
    } else {
        false
    }
}

fn typed_excludes(e: &PlanExpression, c: &str, k: &str) -> bool {
    let different = |a: &PlanExpression, b: &PlanExpression| matches!((a,b),(PlanExpression::Column(n),PlanExpression::Literal(PlanConstant::Text(t))) if n.column.0.as_ref()==c && t.0.as_ref()!=k);
    match e {
        PlanExpression::Compare(p) if p.comparator == Comparator::Equal => {
            different(&p.left, &p.right) || different(&p.right, &p.left)
        }
        PlanExpression::And(p) => typed_excludes(&p.left, c, k) || typed_excludes(&p.right, c, k),
        PlanExpression::Or(p) => typed_excludes(&p.left, c, k) && typed_excludes(&p.right, c, k),
        _ => false,
    }
}
