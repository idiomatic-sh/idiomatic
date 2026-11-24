//! Run-local operator parameters. Dependencies live in the numbered slot graph.
//! Only operators with named parameters ask for this descriptor; select trees stay typed.
use super::R;
use ryo_values::domain::finish_plans::expressions::types::{PlanConstant, PlanNumber};
use std::collections::BTreeMap;
use std::ops::Index;

#[derive(Debug)]
pub(crate) enum Param {
    Missing,
    Text(String),
    Number(PlanNumber),
    List(Vec<Param>),
    Named(BTreeMap<String, Param>),
    CaptureColumns(Vec<Option<usize>>),
}
pub(crate) type Args = Param;
impl Param {
    pub(super) fn text(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
    pub(super) fn strings<'a>(it: impl Iterator<Item = &'a str>) -> Self {
        Self::list(it.map(Self::text).collect())
    }
    pub(super) fn list(items: Vec<Self>) -> Self {
        Self::List(items)
    }
    pub(crate) fn as_str(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    pub(super) fn as_array(&self) -> Option<&[Self]> {
        if let Self::List(xs) = self {
            Some(xs)
        } else {
            None
        }
    }
    pub(super) fn as_object(&self) -> Option<&BTreeMap<String, Self>> {
        if let Self::Named(xs) = self {
            Some(xs)
        } else {
            None
        }
    }
    pub(super) fn as_i64(&self) -> Option<i64> {
        if let Self::Number(PlanNumber::Whole(n)) = self {
            Some(n.0)
        } else {
            None
        }
    }
    pub(super) fn as_u64(&self) -> Option<u64> {
        self.as_i64()?.try_into().ok()
    }
    pub(super) fn get(&self, k: &str) -> Option<&Self> {
        self.as_object()?.get(k)
    }
    pub(super) fn from_constant(c: &PlanConstant) -> Self {
        match c {
            PlanConstant::Text(t) => Self::text(&t.0),
            PlanConstant::Number(n) => Self::Number(*n),
            PlanConstant::List(xs) => Self::list(xs.iter().map(Self::from_constant).collect()),
            PlanConstant::Map(xs) => Self::Named(
                xs.iter()
                    .map(|e| (e.key.0.to_string(), Self::from_constant(&e.value)))
                    .collect(),
            ),
            PlanConstant::Absent => Self::Missing,
        }
    }
}
impl Index<&str> for Param {
    type Output = Self;
    fn index(&self, k: &str) -> &Self {
        self.get(k).unwrap_or(&Self::Missing)
    }
}
impl Index<usize> for Param {
    type Output = Self;
    fn index(&self, i: usize) -> &Self {
        self.as_array()
            .and_then(|a| a.get(i))
            .unwrap_or(&Self::Missing)
    }
}
pub(super) fn s(v: &Param) -> R<&str> {
    v.as_str().ok_or_else(|| format!("not a text: {v:?}"))
}
