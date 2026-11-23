//! Necessary comparisons with values drawn from the project's row for this file.
use super::*;
use ryo_values::domain::finish_plans::arguments::joins::types::AntijoinArguments;

fn origins(a: Attribute, facts: &[Fact]) -> Option<(usize, u64)> {
    match a {
        Attribute::Cell(slot, col)
            if col < 64
                && facts[slot].family == cost::Family::Project
                && facts[slot].kinds[col] == CellKind::Symbol =>
        {
            Some((slot, 1 << col))
        }
        Attribute::ProjectChoices(slot, cols) => Some((slot, cols)),
        _ => None,
    }
}
pub(super) fn columns(mut mask: u64) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if mask == 0 {
            return None;
        }
        let i = mask.trailing_zeros() as usize;
        mask &= mask - 1;
        Some(i)
    })
}
pub(super) fn common(a: Attribute, b: Attribute, facts: &[Fact]) -> Option<Attribute> {
    if a == b {
        return Some(a);
    }
    let (slot, aa) = origins(a, facts)?;
    let (other, bb) = origins(b, facts)?;
    (slot == other).then_some(Attribute::ProjectChoices(slot, aa | bb))
}
impl Builder<'_> {
    pub(super) fn project_membership(&mut self, a: Column, b: Column) -> Option<(usize, Id)> {
        for (candidate, project) in [(a, b), (b, a)] {
            let (Attribute::Cell(slot, col), Attribute::Cell(set, field)) =
                (candidate.attr, project.attr)
            else {
                continue;
            };
            if self.out.facts[slot].key.is_some()
                && self.out.facts[set].family == cost::Family::Project
                && self.out.facts[slot].kinds[col] == CellKind::Symbol
                && self.out.facts[set].kinds[field] == CellKind::Symbol
            {
                return Some((
                    candidate.var,
                    self.domain(Domain::Member(slot, col, set, field)),
                ));
            }
        }
        None
    }
    // A left row passing the right filter supplies its own matching key and
    // cannot survive. If a union renamed several columns to the tested name,
    // at least one possible origin must fail the filter for this file to pass.
    pub(super) fn rejected_filter(
        &mut self,
        j: &AntijoinArguments,
        steps: &[FinishStep],
        left: &Summary<'_>,
    ) -> Option<Id> {
        let FinishStep::Select(q) = &steps[j.right.0] else {
            return None;
        };
        let [src] = q.sources.as_slice() else {
            return None;
        };
        if src.step != j.left || j.left_on.len() != j.right_on.len() {
            return None;
        }
        let selected =
            |col: &ColumnRead| col.alias == SourceAlias::Unwritten || col.alias == src.alias;
        if !j.left_on.iter().zip(&j.right_on).all(|(l,r)|match &q.columns{
            SelectedColumns::All => l==r,
            SelectedColumns::Items(items)=>items.iter().find(|item|item.name==*r).is_some_and(|item|matches!(&item.value,PlanExpression::Column(c) if c.column==*l && selected(c))),
        }){return None}
        let RowCondition::Tested(PlanExpression::Call(f)) = &q.condition else {
            return None;
        };
        if &*f.function.0 != "matches" {
            return None;
        }
        let [
            PlanExpression::Column(c),
            PlanExpression::Literal(PlanConstant::Text(pattern)),
        ] = f.arguments.as_slice()
        else {
            return None;
        };
        if !selected(c) {
            return None;
        }
        let (slot, cols) = origins(left.column(&c.column.0)?.attr, &self.out.facts)?;
        let re = self.regex(&pattern.0)?;
        let mut any = EMPTY;
        for col in columns(cols) {
            let d = self.domain(Domain::Unmatched(slot, col, re));
            let need = self.need(Need::Some(d));
            any = self.or(any, need);
        }
        Some(any)
    }
}
