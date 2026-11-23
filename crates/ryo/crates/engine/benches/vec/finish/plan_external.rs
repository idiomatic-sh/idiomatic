//! Decode the explicitly supplied plan-file format, including its survivor seeds.
//! Keep native step metadata after the input document is released.
use super::{E, Exec, J, Op, Param, R, Rc, Run, Schema};

#[derive(Debug)]
pub(super) struct ExternalStep {
    pub(super) base: Option<String>,
    pub(super) aliases: Vec<Option<String>>,
    pub(super) probe: Option<usize>,
    pub(super) counted: bool,
    pub(super) exists: bool,
    pub(super) args: Param,
    pub(super) condition: Option<Expr>,
    pub(super) items: R<Option<Vec<(String, Expr)>>>,
    pub(super) report: Option<String>,
}

impl ExternalStep {
    pub(super) fn read(node: &J, report: bool) -> Self {
        let items = match &node["items"] {
            J::Null => Ok(None),
            J::Array(items) => items
                .iter()
                .map(|pair| Ok((text(&pair[0])?.to_owned(), Expr::read(&pair[1]))))
                .collect::<R<_>>()
                .map(Some),
            _ => Err("items".into()),
        };
        Self {
            base: node["rel"].as_str().map(str::to_owned),
            aliases: node["src"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|source| source[1].as_str().map(str::to_owned))
                .collect(),
            probe: node["semi"].as_u64().map(|n| n as usize),
            counted: node.get("count").is_some(),
            exists: node.get("exists").is_some(),
            args: Param::external(&node["args"]),
            condition: (!node["cond"].is_null()).then(|| Expr::read(&node["cond"])),
            items,
            report: report.then(|| node.to_string()),
        }
    }
}

fn text(value: &J) -> R<&str> {
    value.as_str().ok_or_else(|| format!("not a text: {value}"))
}

#[derive(Debug)]
pub(super) enum Expr {
    Column(String, Option<String>),
    Text(String),
    Number(i64),
    List(Vec<Expr>),
    Call(String, Vec<Expr>),
    Compare(Op, Box<Expr>, Box<Expr>),
    Member(bool, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Null(Box<Expr>, bool),
    Seed(Box<Expr>, usize),
    Invalid(String),
}

impl Expr {
    fn read(value: &J) -> Self {
        Self::parse(value).unwrap_or_else(Self::Invalid)
    }
    fn parse(j: &J) -> R<Self> {
        let tag = text(&j[0])?;
        let child = |i: usize| Box::new(Self::read(&j[i]));
        let list = |i: usize| {
            j[i].as_array()
                .map(|items| items.iter().map(Self::read).collect())
                .ok_or("list")
        };
        Ok(match tag {
            "col" => Self::Column(text(&j[1])?.to_owned(), j[2].as_str().map(str::to_owned)),
            "lit" => Self::Text(text(&j[1])?.to_owned()),
            "num" => Self::Number(j[1].as_i64().ok_or("num")?),
            "list" => Self::List(list(1)?),
            "call" => Self::Call(text(&j[1])?.to_owned(), list(2)?),
            "cmp" => Self::Compare(Op::of(text(&j[1])?), child(2), child(3)),
            "in" | "notin" => Self::Member(tag == "notin", child(1), child(2)),
            "and" => Self::And(child(1), child(2)),
            "or" => Self::Or(child(1), child(2)),
            "not" => Self::Not(child(1)),
            "null" => Self::Null(child(1), j[2].as_bool().ok_or("null test")?),
            "seed" => Self::Seed(child(1), j[2].as_u64().ok_or("seed set")? as usize),
            other => return Err(format!("expression {other}")),
        })
    }

    pub(super) fn bind(&self, run: &mut Run, sources: &[(Option<String>, Rc<Schema>)]) -> R<E> {
        let nested = |e: &Expr, run: &mut Run| e.bind(run, sources).map(Box::new);
        Ok(match self {
            Self::Column(name, alias) => Exec::column_expr(run, name, alias.as_deref(), sources)?,
            Self::Text(s) => E::Text(run.sym(s.as_bytes())),
            Self::Number(n) => E::Num(*n),
            Self::List(xs) => E::List(xs.iter().map(|x| x.bind(run, sources)).collect::<R<_>>()?),
            Self::Call(name, xs) => {
                let args = xs.iter().map(|x| x.bind(run, sources)).collect::<R<_>>()?;
                Exec::call_expr(run, name, args)?
            }
            Self::Compare(op, a, b) => {
                let a = a.bind(run, sources)?;
                let b = b.bind(run, sources)?;
                Exec::compare_expr(run, *op, a, b)
            }
            Self::Member(negative, a, b) => {
                let a = a.bind(run, sources)?;
                let b = b.bind(run, sources)?;
                Exec::membership_expr(run, a, b, *negative)
            }
            Self::And(a, b) => E::And(nested(a, run)?, nested(b, run)?),
            Self::Or(a, b) => E::Or(nested(a, run)?, nested(b, run)?),
            Self::Not(a) => E::Not(nested(a, run)?),
            Self::Null(a, test) => E::Null(nested(a, run)?, *test),
            Self::Seed(a, set) => E::Seed(nested(a, run)?, *set),
            Self::Invalid(reason) => return Err(reason.clone()),
        })
    }
}
