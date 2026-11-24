//! Resolve expressions in the lowering's typed select directly against source schemas.
use super::{E, Exec, Op, PNode, R, Rc, Run, Schema, Ty, TextTest, symbol_predicate, glob_pattern};
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
        if name == "lower" && let [E::Text(symbol)] = args.as_slice() {
            return Ok(E::Text(run.lower_symbol(*symbol)));
        }
        match (name, args.get(1)) {
            ("matches", Some(E::Text(t))) if args.len() == 2 => {
                let re = run.re(&String::from_utf8_lossy(run.text(*t)).into_owned())?;
                Ok(E::TextTest(Box::new(args.into_iter().next().unwrap()), TextTest::Regex(re)))
            }
            ("glob_any", Some(E::Text(t))) if args.len() == 2 => Self::glob_test(run, &args[0], *t),
            ("glob_any", Some(E::List(items))) if args.len() == 2 => {
                let mut combined = None;
                for item in items {
                    let E::Text(t) = item else { return Err("glob pattern must be text".into()) };
                    let test = Self::glob_test(run, &args[0], *t)?;
                    combined = Some(match combined {
                        Some(left) => E::Or(Box::new(left), Box::new(test)),
                        None => test,
                    });
                }
                Ok(combined.unwrap_or_else(|| E::Cmp(Op::Eq, Box::new(E::Num(0)), Box::new(E::Num(1)))))
            }
            _ => Ok(E::Call(name.to_owned(), args)),
        }
    }
    fn glob_test(run: &mut Run, subject: &E, pattern: u32) -> R<E> {
        match glob_pattern(&String::from_utf8_lossy(run.text(pattern))) {
            Some(regex) => Ok(E::TextTest(Box::new(subject.clone()), TextTest::Regex(run.re(&regex)?))),
            None => {
                use ryo_matching::inbound::use_cases::matching::{
                    interface::MatchingUseCase, types::GlobPattern, use_case::MatchingUseCaseImpl,
                };
                let glob = if let Some(glob) = run.prepared_globs.get(&pattern) {
                    Rc::clone(glob)
                } else {
                    let text = String::from_utf8_lossy(run.text(pattern));
                    let matcher = MatchingUseCaseImpl::new().compile_text_globs(&[GlobPattern(&text)]);
                    let glob = Rc::new(symbol_predicate::PreparedGlob::new(pattern, matcher));
                    run.prepared_globs.insert(pattern, Rc::clone(&glob));
                    glob
                };
                Ok(E::TextTest(Box::new(subject.clone()), TextTest::Glob(glob)))
            }
        }
    }
    // A symbol without a source spelling compares as empty text. Its emitted identity is preserved.
    pub(super) fn compare_expr(run: &mut Run, op: Op, left: E, right: E) -> E {
        if matches!(op, Op::Eq | Op::Ne)
            && let Some(test) = symbol_predicate::folded_literal(run, &left, &right)
                .or_else(|| symbol_predicate::folded_literal(run, &right, &left))
        {
            return if op == Op::Eq { test } else { E::Not(Box::new(test)) };
        }
        let column = match (&left, &right) {
            (col @ E::Col(_, _, Ty::Sym), E::Text(t)) |
            (E::Text(t), col @ E::Col(_, _, Ty::Sym)) if *t == run.empty => Some(col),
            _ => None,
        };
        if let (Some(_), Some(col), Op::Eq | Op::Ne) = (run.absent_word, column, op) {
            return Self::membership_expr(run, col.clone(), E::List(vec![E::Text(run.empty)]), op == Op::Ne);
        }
        E::Cmp(op, Box::new(left), Box::new(right))
    }
    pub(super) fn membership_expr(run: &Run, value: E, mut list: E, negated: bool) -> E {
        if let (Some(absent), E::Col(_, _, Ty::Sym), E::List(items)) = (run.absent_word, &value, &mut list)
            && items.iter().any(|e| matches!(e, E::Text(t) if *t == run.empty))
        {
            items.push(E::Text(absent));
            items.extend(run.unresolved_words.iter().copied().map(E::Text));
        }
        E::In(Box::new(value), Box::new(list), negated)
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
                let left = bind(&c.left)?;
                let right = bind(&c.right)?;
                let op = match c.comparator {
                    Comparator::Equal => Op::Eq,
                    Comparator::NotEqual => Op::Ne,
                    Comparator::Less => Op::Lt,
                    Comparator::LessOrEqual => Op::Le,
                    Comparator::Greater => Op::Gt,
                    Comparator::GreaterOrEqual => Op::Ge,
                };
                Self::compare_expr(run, op, left, right)
            }
            In(p) | NotIn(p) => {
                let value = bind(&p.left)?;
                let list = bind(&p.right)?;
                Self::membership_expr(run, value, list, matches!(e, NotIn(_)))
            }
            And(p) => E::And(Box::new(bind(&p.left)?), Box::new(bind(&p.right)?)),
            Or(p) => E::Or(Box::new(bind(&p.left)?), Box::new(bind(&p.right)?)),
            Not(p) => E::Not(Box::new(bind(p)?)),
        })
    }
}
