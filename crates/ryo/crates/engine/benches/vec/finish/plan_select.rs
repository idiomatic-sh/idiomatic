//! Resolve expressions in the lowering's typed select directly against source schemas.
use super::{E, Exec, Op, PNode, R, Rc, Run, Schema, Ty, glob_pattern};
use ryo_values::domain::finish_plans::{expressions::types::*, types::*};
type Sources = [(Option<String>, Rc<Schema>)];

fn alias(a: &SourceAlias) -> Option<&str> {
    match a {
        SourceAlias::Written(n) => Some(&n.0),
        SourceAlias::Unwritten => None,
    }
}
impl PNode {
    pub(super) fn alias(&self, i: usize) -> Option<&str> {
        if let FinishStep::Select(s) = self.step() {
            alias(&s.sources[i].alias)
        } else {
            None
        }
    }
}
impl Exec<'_> {
    pub(super) fn select_condition(
        &self,
        run: &mut Run,
        n: &PNode,
        sources: &Sources,
    ) -> R<Option<E>> {
        if let FinishStep::Select(s) = n.step() {
            return match &s.condition {
                RowCondition::Every => Ok(None),
                RowCondition::Tested(e) => self.typed_expr(run, e, sources).map(Some),
            };
        }
        Err("not a select".into())
    }
    pub(super) fn select_items(
        &self,
        run: &mut Run,
        n: &PNode,
        sources: &Sources,
    ) -> R<Option<Vec<(String, E)>>> {
        if let FinishStep::Select(s) = n.step() {
            return match &s.columns {
                SelectedColumns::All => Ok(None),
                SelectedColumns::Items(items) => items
                    .iter()
                    .map(|i| {
                        Ok((
                            i.name.0.to_string(),
                            self.typed_expr(run, &i.value, sources)?,
                        ))
                    })
                    .collect::<R<_>>()
                    .map(Some),
            };
        }
        Err("not a select".into())
    }
    pub(super) fn column_expr(
        run: &mut Run,
        name: &str,
        qualifier: Option<&str>,
        sources: &Sources,
    ) -> R<E> {
        for (i, (al, sc)) in sources.iter().enumerate() {
            if qualifier.is_some()
                && al.as_deref() != qualifier
                && !(sources.len() == 1 && al.is_none())
            {
                continue;
            }
            if let Some(c) = sc.names.iter().position(|x| x == name) {
                return Ok(E::Col(i, c, sc.tys[c]));
            }
        }
        if qualifier.is_none() {
            Ok(E::Text(run.sym(name.as_bytes())))
        } else {
            Err(format!("no column {qualifier:?}.{name}"))
        }
    }
    pub(super) fn call_expr(run: &mut Run, name: &str, args: Vec<E>) -> R<E> {
        match (name, args.get(1)) {
            ("matches", Some(E::Text(t))) if args.len() == 2 => {
                let re = run.re(&String::from_utf8_lossy(run.text(*t)).into_owned())?;
                Ok(E::Re(Box::new(args.into_iter().next().unwrap()), re))
            }
            ("glob_any", Some(E::Text(t))) if args.len() == 2 => {
                let re = run.re(&glob_pattern(&String::from_utf8_lossy(run.text(*t))))?;
                Ok(E::Re(Box::new(args.into_iter().next().unwrap()), re))
            }
            ("glob_any", Some(E::List(items))) if args.len() == 2 => {
                let mut tests = items
                    .iter()
                    .map(|item| {
                        let E::Text(t) = item else {
                            return Err("glob pattern must be text".to_owned());
                        };
                        let re = run.re(&glob_pattern(&String::from_utf8_lossy(run.text(*t))))?;
                        Ok(E::Re(Box::new(args[0].clone()), re))
                    })
                    .collect::<R<Vec<_>>>()?
                    .into_iter();
                Ok(match tests.next() {
                    Some(first) => {
                        tests.fold(first, |left, right| E::Or(Box::new(left), Box::new(right)))
                    }
                    None => E::Cmp(Op::Eq, Box::new(E::Num(0)), Box::new(E::Num(1))),
                })
            }
            _ => Ok(E::Call(name.to_owned(), args)),
        }
    }
    fn typed_expr(&self, run: &mut Run, e: &PlanExpression, sources: &Sources) -> R<E> {
        use PlanExpression::*;
        let mut bind = |e: &PlanExpression| self.typed_expr(run, e, sources);
        Ok(match e {
            Column(c) => Self::column_expr(run, &c.column.0, alias(&c.alias), sources)?,
            Literal(PlanConstant::Text(t)) => E::Text(run.sym(t.0.as_bytes())),
            Literal(other) => return Err(format!("not a text: {other:?}")),
            Number(PlanNumber::Whole(n)) => E::Num(n.0),
            Number(_) => return Err("num".into()),
            List(xs) => E::List(xs.iter().map(bind).collect::<R<_>>()?),
            Call(c) => {
                let args = c.arguments.iter().map(bind).collect::<R<_>>()?;
                Self::call_expr(run, &c.function.0, args)?
            }
            Compare(c) => {
                let op = match c.comparator {
                    Comparator::Equal => Op::Eq,
                    Comparator::NotEqual => Op::Ne,
                    Comparator::Less => Op::Lt,
                    Comparator::LessOrEqual => Op::Le,
                    Comparator::Greater => Op::Gt,
                    Comparator::GreaterOrEqual => Op::Ge,
                };
                let left = bind(&c.left)?;
                let right = bind(&c.right)?;
                // SDK literal comparisons use the spelling. Both an unpublished
                // symbol and a published empty string have the empty spelling.
                let column = match (&left, &right) {
                    (column @ E::Col(_, _, Ty::Sym), E::Text(t))
                    | (E::Text(t), column @ E::Col(_, _, Ty::Sym))
                        if *t == run.empty =>
                    {
                        Some(column)
                    }
                    _ => None,
                };
                if let (Some(absent), Some(column), Op::Eq | Op::Ne) = (run.absent_word, column, op)
                {
                    E::In(
                        Box::new(column.clone()),
                        Box::new(E::List(vec![E::Text(run.empty), E::Text(absent)])),
                        op == Op::Ne,
                    )
                } else {
                    E::Cmp(op, Box::new(left), Box::new(right))
                }
            }
            In(p) | NotIn(p) => {
                let value = bind(&p.left)?;
                let mut list = bind(&p.right)?;
                if let (Some(absent), E::Col(_, _, Ty::Sym), E::List(items)) =
                    (run.absent_word, &value, &mut list)
                    && items
                        .iter()
                        .any(|item| matches!(item, E::Text(t) if *t == run.empty))
                {
                    items.push(E::Text(absent));
                }
                E::In(Box::new(value), Box::new(list), matches!(e, NotIn(_)))
            }
            And(p) => E::And(Box::new(bind(&p.left)?), Box::new(bind(&p.right)?)),
            Or(p) => E::Or(Box::new(bind(&p.left)?), Box::new(bind(&p.right)?)),
            Not(p) => E::Not(Box::new(bind(p)?)),
        })
    }
}
