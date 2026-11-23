//! Columns read by the Python provider's composed facts. The answer also
//! includes any columns requested by this file's operators.
use super::{fact_catalog as facts, finish};

const TYPE: u8 = 1;
const ANNOTATIONS: u8 = 2;
const RESOLUTION: u8 = 4;
const FINAL: u8 = 8;
const DEFERRED: u8 = 16;

const VISITS: u8 = 32;
const STUB_TYPES: u8 = 64;

const INPUTS: [(&str, u8, &[&str]); 13] = [
    ("qualified-name", TYPE, &["id", "path"]),
    ("identifier", TYPE, &["id", "name"]),
    ("call-argument", TYPE, &["call", "expression", "position"]),
    ("embedded-expression", TYPE | ANNOTATIONS, &["root"]),
    ("compiler-feature", ANNOTATIONS, &["feature", "enabled"]),
    ("language-target", ANNOTATIONS | DEFERRED, &["major", "minor"]),
    (facts::TYPE_EXPRESSION_REL, STUB_TYPES, &["id", "kind"]),
    (facts::ANNOTATION_EVALUATION_REL, VISITS, &["id", "evaluation"]),
    (facts::DEFERRED_VISIT_REL, DEFERRED, &["id", "visit"]),
    // The binding writers own their input layout. Hand them whole tables.
    ("binding-resolution", RESOLUTION, &[]),
    ("binding-read", RESOLUTION | DEFERRED, &[]),
    ("binding-event", FINAL | DEFERRED, &[]),
    ("implicit-symbol", DEFERRED, &[]),
];

#[derive(Clone, Copy)]
pub(super) struct Layout([u64; INPUTS.len()]);

impl Layout {
    pub(super) fn from_catalog(run: &mut finish::Run) -> Self {
        Self(INPUTS.map(|(rel, _, fields)| {
            if fields.is_empty() { return u64::MAX; }
            let Some((_, names, _)) = run.catalog(rel) else { return u64::MAX };
            fields.iter().try_fold(0, |mask, field| {
                let c = names.iter().position(|n| n.as_str() == *field)?;
                1u64.checked_shl(c as u32).map(|bit| mask | bit)
            }).unwrap_or(u64::MAX)
        }))
    }

    pub(super) fn for_file(self, entries: &[finish::request::Asked<'_>], stub: bool) -> Columns {
        let mut active = 0;
        for ask in entries {
            active |= match ask.rel {
                facts::TYPE_EXPRESSION_REL => TYPE,
                facts::ANNOTATION_EVALUATION_REL => ANNOTATIONS,
                facts::RESOLUTION_REL => RESOLUTION,
                facts::FINAL_REL => FINAL,
                facts::DEFERRED_VISIT_REL => ANNOTATIONS | VISITS | if stub { TYPE | STUB_TYPES } else { 0 },
                facts::DEFERRED_RESOLUTION_REL => DEFERRED | FINAL | ANNOTATIONS | VISITS | if stub { TYPE | STUB_TYPES } else { 0 },
                _ => 0,
            };
        }
        let mut result = Columns::default();
        for (i, (_, used, _)) in INPUTS.iter().enumerate() {
            if active & used != 0 { result.0[i] = Some(self.0[i]); }
        }
        for ask in entries {
            if let Some(index) = input(ask.rel) {
                if matches!(ask.rel, facts::TYPE_EXPRESSION_REL | facts::ANNOTATION_EVALUATION_REL | facts::DEFERRED_VISIT_REL) {
                    *result.0[index].get_or_insert(0) |= ask.columns.word(0);
                } else if let Some(mask) = &mut result.0[index] {
                    *mask |= ask.columns.word(0);
                }
            }
        }
        result
    }
}

fn input(rel: &str) -> Option<usize> {
    INPUTS.iter().position(|(name, _, _)| *name == rel)
}

#[derive(Default)]
pub(super) struct Columns([Option<u64>; INPUTS.len()]);
impl Columns {
    pub(super) fn get(&self, rel: &str) -> Option<u64> {
        input(rel).and_then(|index| self.0[index])
    }
}

/// Intermediate tables have catalog names fixed by the composed writers.
/// Only those inputs can be reused while building this file's single answer.
#[derive(Default)]
pub(super) struct Tables([Option<std::rc::Rc<finish::Table>>; INPUTS.len() + 3]);

impl Tables {
    fn slot(rel: &str) -> Option<usize> {
        match rel {
            "lookup-scope" => Some(INPUTS.len()),
            facts::FINAL_REL => Some(INPUTS.len() + 1),
            facts::DEFERRED_RESOLUTION_REL => Some(INPUTS.len() + 2),
            _ => input(rel),
        }
    }

    pub(super) fn get(&self, rel: &str) -> Option<&std::rc::Rc<finish::Table>> {
        self.0[Self::slot(rel)?].as_ref()
    }

    pub(super) fn insert(&mut self, rel: &str, table: std::rc::Rc<finish::Table>) {
        if let Some(i) = Self::slot(rel) { self.0[i] = Some(table); }
    }

    pub(super) fn clear(&mut self) { self.0.fill(None); }
}
