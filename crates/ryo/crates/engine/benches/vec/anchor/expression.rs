//! Bind cell positions and operand types once, including short-circuit predicates.
use super::*;
type Get = Box<dyn Fn(&mut Run, &[u64], &[u64]) -> V>;

pub(super) struct Select {
    pub tests: Vec<Get>,
    pub items: Vec<Read>,
}
pub(super) enum Read {
    Column(usize, usize),
    Value(Get),
}
impl Read {
    pub fn get(&self, run: &mut Run, a: &[u64], b: &[u64]) -> u64 {
        match self {
            Self::Column(0, c) => a[*c],
            Self::Column(_, c) => b[*c],
            Self::Value(f) => f(run, a, b).1,
        }
    }
}
impl Select {
    pub fn bind(sp: &SelPlan) -> Option<Self> {
        let items = sp
            .items
            .iter()
            .map(|(_, e)| {
                Some(match e {
                    E::Col(side, c, _) => Read::Column(*side, *c),
                    _ => Read::Value(bind(e)?),
                })
            })
            .collect::<Option<_>>()?;
        Some(Self {
            tests: sp.conj.iter().map(bind).collect::<Option<_>>()?,
            items,
        })
    }
}
fn bind(e: &E) -> Option<Get> {
    use std::cmp::Ordering::*;
    Some(match e {
        E::Col(s, c, t) => {
            let (s, c, t) = (*s, *c, *t);
            Box::new(move |_, a, b| V::new(t, if s == 0 { a[c] } else { b[c] }))
        }
        E::Lit(v) => {
            let v = *v;
            Box::new(move |_, _, _| v)
        }
        E::Text(v) => {
            let v = V::new(Ty::Sym, u64::from(*v));
            Box::new(move |_, _, _| v)
        }
        E::Num(v) => {
            let v = V::new(Ty::Num, *v as u64);
            Box::new(move |_, _, _| v)
        }
        E::And(x, y) | E::Or(x, y) => {
            let and = matches!(e, E::And(..));
            let x = bind(x)?;
            let y = bind(y)?;
            Box::new(move |r, a, b| {
                if truthy(x(r, a, b)) == and {
                    flag(truthy(y(r, a, b)))
                } else {
                    flag(!and)
                }
            })
        }
        E::Not(x) => {
            let x = bind(x)?;
            Box::new(move |r, a, b| flag(!truthy(x(r, a, b))))
        }
        E::Null(x, n) => {
            let x = bind(x)?;
            let n = *n;
            Box::new(move |r, a, b| flag((x(r, a, b).1 == NULL) != n))
        }
        E::Cmp(op, x, y) => {
            let op = *op;
            let same = matches!((&**x,&**y),(E::Col(_,_,a),E::Col(_,_,b)) if a==b);
            let sym = matches!(
                (&**x, &**y),
                (E::Col(_, _, Ty::Sym), E::Text(_)) | (E::Text(_), E::Col(_, _, Ty::Sym))
            );
            let x = bind(x)?;
            let y = bind(y)?;
            if (same || sym) && matches!(op, Op::Eq | Op::Ne) {
                Box::new(move |r, a, b| {
                    let v = x(r, a, b).1;
                    flag((v == y(r, a, b).1 && v != NULL) == (op == Op::Eq))
                })
            } else {
                Box::new(move |r, a, b| {
                    let l = x(r, a, b);
                    let v = y(r, a, b);
                    flag(compare(r, l, v).is_some_and(|c| match op {
                        Op::Eq => c == Equal,
                        Op::Ne => c != Equal,
                        Op::Lt => c == Less,
                        Op::Le => c != Greater,
                        Op::Gt => c == Greater,
                        Op::Ge => c != Less,
                        Op::Other => false,
                    }))
                })
            }
        }
        E::In(x, y, neg) => {
            let E::List(vs) = &**y else { return None };
            let x = bind(x)?;
            let ys = vs.iter().map(bind).collect::<Option<Vec<_>>>()?;
            let neg = *neg;
            Box::new(move |r, a, b| {
                let v = x(r, a, b);
                flag(
                    ys.iter().any(|y| {
                        let t = y(r, a, b);
                        compare(r, v, t) == Some(Equal)
                    }) != neg,
                )
            })
        }
        E::Re(x, re) => {
            let x = bind(x)?;
            let re = re.clone();
            Box::new(move |r, a, b| {
                let v = x(r, a, b);
                flag(re.is_match(&text_of(r, v)))
            })
        }
        E::Call(name, es) => {
            let name = name.clone();
            let args = es.iter().map(bind).collect::<Option<Vec<_>>>()?;
            Box::new(move |r, a, b| {
                let vs = args
                    .iter()
                    .map(|v| v(r, a, b))
                    .collect::<smallvec::SmallVec<[V; 4]>>();
                call(r, &name, &vs)
            })
        }
        _ => return None,
    })
}
