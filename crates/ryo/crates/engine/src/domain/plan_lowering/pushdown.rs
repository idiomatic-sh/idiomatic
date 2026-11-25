//! Move a total row predicate across a direct projection of an inner equijoin.

use super::readers::{
    service::conjuncts,
    types::{Emitted, Uses},
};
use ryo_values::domain::finish_plans::{
    expressions::types::{
        ColumnRead, Comparator, ExpressionPair, PlanConstant, PlanExpression as Expr,
        RowComparison, SourceAlias,
    },
    types::{
        CountMark, FinishPlan, FinishStep, ProbeMark, RowCondition, SelectStep, SelectedColumns,
    },
};

pub(super) fn mark(plan: &mut FinishPlan, uses: &Uses) {
    for number in 0..plan.steps.len() {
        if uses.emitted[number] == Emitted::Yes {
            continue;
        }
        let [reader] = uses.readers[number].as_slice() else {
            continue;
        };
        let (FinishStep::Select(join), FinishStep::Select(filter)) =
            (&plan.steps[number], &plan.steps[reader.0])
        else {
            continue;
        };
        let Some(test) = necessary(join, filter) else {
            continue;
        };
        if let FinishStep::Select(join) = &mut plan.steps[number] {
            join.probe = ProbeMark::Filtered(Box::new(test));
        }
    }
}

fn necessary(join: &SelectStep, filter: &SelectStep) -> Option<Expr> {
    let [left, right] = join.sources.as_slice() else {
        return None;
    };
    let [outer] = filter.sources.as_slice() else {
        return None;
    };
    if join.probe != ProbeMark::Unmarked || join.count != CountMark::Unmarked {
        return None;
    }
    let (SourceAlias::Written(_), SourceAlias::Written(_)) = (&left.alias, &right.alias) else {
        return None;
    };
    if left.alias == right.alias {
        return None;
    }
    let (RowCondition::Tested(keys), RowCondition::Tested(test), SelectedColumns::Items(items)) =
        (&join.condition, &filter.condition, &join.columns)
    else {
        return None;
    };
    // Match the native ordered single-key join. Residual conditions use the ordinary select.
    let Expr::Compare(RowComparison {
        comparator: Comparator::Equal,
        left: a,
        right: b,
    }) = keys.as_ref()
    else {
        return None;
    };
    let (Expr::Column(a), Expr::Column(b)) = (&**a, &**b) else {
        return None;
    };
    if !((&a.alias == &left.alias && &b.alias == &right.alias)
        || (&a.alias == &right.alias && &b.alias == &left.alias))
    {
        return None;
    }

    // A rejected row must not suppress a call or another non-scalar expression
    // in the original consumer, whose order the executor may already change.
    if !total(test) {
        return None;
    }

    // A conjunction permits a local necessary test even if another conjunct
    // reads a different source. Keep the original filter as the logical consumer.
    for conjunct in conjuncts(test) {
        let mut source = None;
        let mut resolve = |read: &ColumnRead| {
            if read.alias != SourceAlias::Unwritten && read.alias != outer.alias {
                return None;
            }
            let mut origins = items.iter().filter(|item| item.name == read.column);
            let Expr::Column(origin) = &origins.next()?.value else {
                return None;
            };
            if origins.next().is_some()
                || (origin.alias != left.alias && origin.alias != right.alias)
            {
                return None;
            }
            if source.is_some_and(|alias| alias != &origin.alias) {
                return None;
            }
            source = Some(&origin.alias);
            Some(Expr::Column(origin.clone()))
        };
        if let Some(test) = translated(conjunct, &mut resolve)
            && source.is_some()
        {
            return Some(test);
        }
    }
    None
}

fn total(expr: &Expr) -> bool {
    match expr {
        Expr::Column(_)
        | Expr::Number(_)
        | Expr::Literal(PlanConstant::Text(_) | PlanConstant::Absent) => true,
        Expr::Compare(c) => total(&c.left) && total(&c.right),
        Expr::In(p) | Expr::NotIn(p) | Expr::And(p) | Expr::Or(p) => {
            total(&p.left) && total(&p.right)
        }
        Expr::Not(x) => total(x),
        Expr::List(xs) => xs.iter().all(total),
        Expr::Call(_) | Expr::Literal(_) => false,
    }
}

fn translated(expr: &Expr, resolve: &mut impl FnMut(&ColumnRead) -> Option<Expr>) -> Option<Expr> {
    let pair = |p: &ExpressionPair, resolve: &mut _| {
        Some(ExpressionPair {
            left: Box::new(translated(&p.left, resolve)?),
            right: Box::new(translated(&p.right, resolve)?),
        })
    };
    Some(match expr {
        Expr::Column(read) => return resolve(read),
        Expr::Number(_) | Expr::Literal(PlanConstant::Text(_) | PlanConstant::Absent) => {
            expr.clone()
        }
        Expr::Compare(c) => Expr::Compare(RowComparison {
            comparator: c.comparator,
            left: Box::new(translated(&c.left, resolve)?),
            right: Box::new(translated(&c.right, resolve)?),
        }),
        Expr::In(p) => Expr::In(pair(p, resolve)?),
        Expr::NotIn(p) => Expr::NotIn(pair(p, resolve)?),
        Expr::And(p) => Expr::And(pair(p, resolve)?),
        Expr::Or(p) => Expr::Or(pair(p, resolve)?),
        Expr::Not(x) => Expr::Not(Box::new(translated(x, resolve)?)),
        Expr::List(xs) => Expr::List(
            xs.iter()
                .map(|x| translated(x, resolve))
                .collect::<Option<_>>()?,
        ),
        Expr::Call(_) | Expr::Literal(_) => return None,
    })
}
