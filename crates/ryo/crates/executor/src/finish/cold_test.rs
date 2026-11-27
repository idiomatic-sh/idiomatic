//! Compile a base-view field test once for the run's provider vocabulary.
use super::{File, Native, Run, ViewSpec};
use rustc_hash::FxHashMap;
use std::collections::BTreeMap;
use std::rc::Rc;

pub enum Kernel {
    Columns(&'static str, super::cold_pred::Program),
    One(u16),
    FieldWords(Box<[u32]>),
    Set { allowed: Box<[bool]>, unknown: bool },
}

impl Kernel {
    pub fn fields(run: &Run, members: &[u32]) -> Self {
        if run.file_field_codes {
            return Self::FieldWords(members.into());
        }
        let unknown = members.binary_search(&run.empty).is_ok();
        let allowed: Box<[bool]> = run
            .field_sym
            .iter()
            .map(|word| members.binary_search(word).is_ok())
            .collect();
        let mut yes = allowed.iter().enumerate().filter(|(_, accept)| **accept);
        if !unknown
            && let Some((code, _)) = yes.next()
            && yes.next().is_none()
            && let Ok(code) = u16::try_from(code)
        {
            return Self::One(code);
        }
        Self::Set { allowed, unknown }
    }

    pub fn retain(&self, f: &File<'_>, run: &mut Run, rows: &mut Vec<u32>) {
        let field = &f.field;
        match self {
            Self::Columns(_, program) => program.retain(f, run, rows),
            Self::FieldWords(words) => rows.retain(|r| {
                let word = run
                    .field_sym
                    .get(usize::from(field[*r as usize]))
                    .copied()
                    .unwrap_or(run.empty);
                words.binary_search(&word).is_ok()
            }),
            Self::One(code) => rows.retain(|r| field[*r as usize] == *code),
            Self::Set { allowed, unknown } => rows.retain(|r| {
                allowed
                    .get(usize::from(field[*r as usize]))
                    .copied()
                    .unwrap_or(*unknown)
            }),
        }
    }
}

// Positive literals in the same column of a conjunction intersect. Provider codes
// with the same symbol (and out-of-vocabulary fields) keep their original meaning.
pub fn field_members(v: &ViewSpec) -> Option<Vec<u32>> {
    if !v.base.is_child() {
        return None;
    }
    let mut exprs = v.filter.iter();
    let mut allowed = super::cold_list::members(exprs.next()?, 3)?;
    for e in exprs {
        let other = super::cold_list::members(e, 3)?;
        allowed.retain(|x| other.contains(x));
    }
    allowed.sort_unstable();
    allowed.dedup();
    Some(allowed)
}

/// Filters shared by views first reached during this run. SDK field codes are resolved per file.
#[derive(Default)]
pub struct Compiler {
    pub fields: FxHashMap<Vec<u32>, Rc<Kernel>>,
    pub programs: FxHashMap<(Native, Vec<super::cold_list::FilterKey>), Option<Rc<Kernel>>>,
    pub inventory: BTreeMap<(&'static str, bool), (usize, usize)>,
}

impl Compiler {
    pub fn bind(&mut self, run: &mut Run, v: &ViewSpec) {
        if v.filter.is_empty() {
            return;
        }
        if let Some(entry) = super::cold_list::Handoff::compile(run, v) {
            assert!(v.cold_bucket.set(entry).is_ok());
        }
        let form = super::cold_list::form(v);
        let mut first = false;
        let kernel = if let Some(words) = field_members(v) {
            Some(
                self.fields
                    .entry(words)
                    .or_insert_with_key(|words| {
                        first = true;
                        Rc::new(Kernel::fields(run, words))
                    })
                    .clone(),
            )
        } else {
            let key = (v.base, super::cold_list::conjuncts(&v.filter));
            self.programs
                .entry(key)
                .or_insert_with(|| {
                    first = true;
                    super::cold_pred::Program::compile(run, v)
                        .map(|p| Rc::new(Kernel::Columns(form, p)))
                })
                .clone()
        };

        if let Some(k) = kernel {
            v.cold_test
                .set(k)
                .unwrap_or_else(|_| panic!("cold test set twice"));
        }
    }
}

/// Gate runs apply the original filter to a copy and compare all row IDs, in order.
pub fn retain(
    f: &File<'_>,
    run: &mut Run,
    v: &ViewSpec,
    rows: &mut Vec<u32>,
) {
    match v.cold_test.get() {
        Some(kernel) => {
            kernel.retain(f, run, rows);
        }
        None => {
            if !v.filter.is_empty() {
                super::view_cols::retain(f, run, v, rows);
            }
        }
    }
}

/// Read a typed base column or its already indexed literal rows directly.
pub fn column_rows(
    f: &File<'_>,
    run: &mut Run,
    v: &ViewSpec,
) -> Option<(Rc<Vec<u32>>, usize)> {
    let Kernel::Columns(_, program) = &**v.cold_test.get()? else {
        return None;
    };
    let (rows, entered, _) = program.rows(f, run)?;

    Some((rows, entered))
}

/// The select front can enter a narrower base bucket and apply the very same view filter.
pub fn entry(f: &File<'_>, run: &mut Run, v: &ViewSpec, rows: &mut Vec<u32>) -> bool {
    let Some(kernel) = v.cold_test.get() else {
        return false;
    };
    {
        kernel.retain(f, run, rows);
    }
    true
}

