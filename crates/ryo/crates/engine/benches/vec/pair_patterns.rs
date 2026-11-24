//! A disjunction of globs on one column remains a single membership test.
//! Actual syntax spellings come from the reader, including derived symbols.
use super::{
    Attribute, Builder, CAP, Column, ColumnRead, Domain, EMPTY, Id, PlanConstant, PlanExpression,
};

// Count a typed list as one predicate. Bound its stored literals separately
// so a large caller parameter can always widen to an unconstrained check.
const GLOB_LITERAL_BYTES: usize = 16 * 1024;

fn gather<'a>(
    e: &'a PlanExpression,
    patterns: &mut Vec<&'a str>,
    calls: &mut usize,
    bytes: &mut usize,
) -> Option<&'a ColumnRead> {
    use PlanExpression::*;
    match e {
        Or(p) => {
            let a = gather(&p.left, patterns, calls, bytes)?;
            let b = gather(&p.right, patterns, calls, bytes)?;
            (a == b).then_some(a)
        }
        Call(f) if &*f.function.0 == "glob_any" => {
            let [Column(c), pattern] = f.arguments.as_slice() else {
                return None;
            };
            *calls = calls.checked_sub(1)?;
            let mut add = |t: &'a str| {
                *bytes = bytes.checked_sub(t.len().checked_add(1)?)?;
                patterns.push(t);
                Some(())
            };
            match pattern {
                Literal(PlanConstant::Text(t)) => add(&t.0)?,
                List(items) => {
                    for i in items {
                        let Literal(PlanConstant::Text(t)) = i else {
                            return None;
                        };
                        add(&t.0)?;
                    }
                }
                _ => return None,
            }
            Some(c)
        }
        _ => None,
    }
}
fn glob_terms(e: &PlanExpression) -> Option<(&ColumnRead, Vec<&str>)> {
    let mut literals = Vec::new();
    let mut calls = CAP;
    let mut bytes = GLOB_LITERAL_BYTES;
    let column = gather(e, &mut literals, &mut calls, &mut bytes)?;
    Some((column, literals))
}

impl Builder<'_> {
    /// After a union a symbol may come from either supplied column. Keep
    /// both origins for file-level existence only; their node keys may differ.
    pub(super) fn common_origin(&mut self, left: Attribute, right: Attribute) -> Option<Attribute> {
        if let Some(origin) = super::project::common(left, right, &self.out.facts) {
            return Some(origin);
        }
        let mut origins = Vec::new();
        for value in [left, right] {
            match value {
                Attribute::Cell(slot, column)
                    if self.out.facts[slot].kinds[column] == super::CellKind::Symbol =>
                {
                    origins.push((slot, column))
                }
                Attribute::SymbolOrigins(i) => origins.extend_from_slice(&self.symbol_origins[i]),
                _ => return None,
            }
        }
        origins.sort_unstable();
        origins.dedup();
        if origins.len() > CAP {
            return None;
        }
        let index = self
            .symbol_origins
            .iter()
            .position(|v| v == &origins)
            .unwrap_or_else(|| {
                let index = self.symbol_origins.len();
                self.symbol_origins.push(origins);
                index
            });
        Some(Attribute::SymbolOrigins(index))
    }
    pub(super) fn glob_test(
        &mut self,
        e: &PlanExpression,
        lookup: &impl Fn(&ColumnRead) -> Option<Column>,
    ) -> Option<(Column, usize, bool)> {
        let (e, positive) = match e {
            PlanExpression::Not(x) => (x.as_ref(), false),
            _ => (e, true),
        };
        let (column, patterns) = glob_terms(e)?;
        let col = lookup(column)?;
        if patterns.is_empty() {
            return None;
        }
        let pattern = patterns
            .iter()
            .map(|s| crate::glob_pattern(s))
            .collect::<Vec<_>>()
            .join("|");
        let re = self.regex(&pattern)?;
        Some((col, re, positive))
    }
    pub(super) fn glob_domain(
        &mut self,
        e: &PlanExpression,
        lookup: &impl Fn(&ColumnRead) -> Option<Column>,
    ) -> Option<(usize, Id)> {
        // A plain glob is exact equality on this column. Use the shared literal
        // postings, including when the inputs spell several alternatives.
        if let Some((column, globs)) = glob_terms(e)
            && let Some(col) = lookup(column)
            && matches!(col.attr, Attribute::Text | Attribute::Cell(..))
        {
            let mut domain = EMPTY;
            let mut complete = true;
            let mut remaining = CAP;
            for glob in globs {
                let alternatives = glob.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
                if alternatives.is_none() && glob.contains(',') {
                    complete = false;
                    break;
                }
                for value in alternatives.unwrap_or(glob).split(',') {
                    if remaining == 0 || value.contains(['{', '}', '*', '?']) || value.is_empty() {
                        complete = false;
                        break;
                    }
                    remaining -= 1;
                    let d = self.literal(col.attr, value);
                    domain = self.either(domain, d);
                }
                if !complete {
                    break;
                }
            }
            if complete {
                return Some((col.var, domain));
            }
        }
        let (col, re, positive) = self.glob_test(e, lookup)?;
        if col.attr != Attribute::Kind {
            return None;
        }
        let names = &self.vocab.kind_names;
        if names.is_empty() {
            return None;
        }
        let regex = self.out.regexes[re].compiled()?;
        let mut codes = Vec::new();
        for (k, value) in names.iter().enumerate() {
            if regex.is_match(value.as_bytes()) == positive {
                if codes.len() == CAP {
                    return None;
                }
                codes.push(u16::try_from(k).ok()?);
            }
        }
        let mut ids = EMPTY;
        for code in codes {
            let d = self.domain(Domain::Kind(code));
            ids = self.either(ids, d);
        }
        Some((col.var, ids))
    }
}
