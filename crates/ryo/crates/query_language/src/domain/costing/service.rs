//! The `costing` service: free functions named by verb, with no IO and no ports. `classified` reads the cost of a query off its
//! steps as the compiler assembled them, `sited` says where a statement is written, `described` words a cost for the one who
//! wrote the query, and `held` checks a pack's rules against the ceiling and the list its manifest declares.

pub(crate) mod joins;
pub(crate) mod steps;

use std::collections::BTreeMap;

use self::steps::service::{refine_join, step_cost};
use ryo_relations::domain::relations::types::{Relation, RelationSize};
use ryo_values::domain::descriptions::types::CostLine;
use ryo_values::domain::program::types::{LetStep, ParsedModules};
use ryo_values::domain::query_tokens::types::QueryOrigin;

use super::types::{
    AssembledSteps, CostBreach, CostFinding, CostedStep, ListedSite, PackCeiling, PlanCost,
    PlanCostClass, RuleCost, ShapeClass, SiteHome, SiteOrigin, SiteOwner, WrittenSite,
};
use crate::domain::distinctness::types::DistinctMemo;
use crate::domain::pruning::service::read_from;
use crate::domain::pruning::types::StepRead;

/// The cost class of a query: a pure function of its steps as assembled and of the sizes the catalog declares, which `sizes`
/// answers (in a compile, `ryo_relations::domain::catalog::service::relation_size`). The steps are read once, in order: a step
/// reads only steps before it, and what is asked of the steps a join reads (whether a side holds each key once) is followed
/// once a step and kept. A step no root reads is not classified, and a statement read more than once is one finding, at the
/// greatest class it is read at.
#[must_use]
pub(crate) fn classified(
    assembled: AssembledSteps<'_>,
    sizes: fn(Relation) -> Option<RelationSize>,
) -> PlanCost {
    let mut kept: Vec<CostedStep> = Vec::with_capacity(assembled.steps.len());
    let mut distinct = DistinctMemo::default();
    for (step, written) in assembled.steps.iter().zip(assembled.written) {
        refine_join(
            assembled,
            sizes,
            &mut kept,
            &step.operation,
            written.as_ref(),
        );
        kept.push(step_cost(assembled, sizes, &mut distinct, &step.operation));
    }
    let read = read_from(assembled.steps, assembled.roots);
    let mut sites: BTreeMap<&WrittenSite, ShapeClass> = BTreeMap::new();
    for ((cost, written), reached) in kept.iter().zip(assembled.written).zip(&read) {
        if let (StepRead::Read, Some(verdict), Some(site)) = (reached, cost.verdict, written) {
            let greatest = sites.entry(site).or_insert(verdict);
            if greatest.class < verdict.class {
                *greatest = verdict;
            }
        }
    }
    let findings: Vec<CostFinding> = sites
        .into_iter()
        .map(|(written, verdict)| CostFinding {
            site: written.site.clone(),
            home: written.home,
            span: written.span,
            shape: verdict.shape,
            class: verdict.class,
        })
        .collect();
    PlanCost {
        class: findings
            .iter()
            .map(|finding| finding.class)
            .max()
            .unwrap_or(PlanCostClass::Linear),
        findings,
    }
}

/// Where the statement an expansion or the query itself wrote is written: the document of its origin, and the name its `let`
/// gives there. For a statement of a module that is the `let` at the origin's span in the module as parsed, never the name the
/// expanded statement carries, which for the statement a module returns is its caller's; for a statement of the query's own
/// document, which is no module, it is the statement's own name. The origin's trace, the path of imports that reached the
/// module, is never read. The statements of a module are in the order of their text, so the `let` is found by halving. The
/// origin holds no line: the site's place is the statement's bytes in its document, from which the document's text gives a line.
#[must_use]
pub(crate) fn sited(
    origin: &QueryOrigin,
    statement: &LetStep,
    modules: &ParsedModules,
) -> WrittenSite {
    let (name, home) = match modules.0.get(&origin.source_id) {
        Some(module) => {
            let place = module
                .steps
                .partition_point(|written| written.span.start < origin.span.start);
            let written = module
                .steps
                .get(place)
                .filter(|written| written.span == origin.span)
                .map_or(statement, |written| written);
            (&written.name.0, SiteHome::Module)
        }
        None => (&statement.name.0, SiteHome::Query),
    };
    WrittenSite {
        site: SiteOrigin {
            document: origin.source_id.clone(),
            statement: name.clone(),
        },
        home,
        span: origin.span,
    }
}

/// A cost in the words `query validate` prints: the class with the statement that sets it, then a line for each other statement
/// above linear with its own class.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::service::described;
/// use ryo_query_language::domain::costing::types::{PlanCost, PlanCostClass};
///
/// let lines = described(&PlanCost { class: PlanCostClass::Linear, findings: Vec::new() });
/// assert_eq!(&*lines[0].0, "linear");
/// ```
#[must_use]
pub fn described(cost: &PlanCost) -> Vec<CostLine> {
    let setter = cost
        .findings
        .iter()
        .position(|finding| finding.class == cost.class);
    let mut lines = vec![CostLine(
        match setter.and_then(|place| cost.findings.get(place)) {
            Some(finding) => format!("{} ({finding})", cost.class),
            None => cost.class.to_string(),
        }
        .into(),
    )];
    for (place, finding) in cost.findings.iter().enumerate() {
        if Some(place) != setter {
            lines.push(CostLine(format!("{} ({finding})", finding.class).into()));
        }
    }
    lines
}

/// How a pack's rules break what its manifest declares; none when they hold it. A rule that reaches a site above the pack's
/// ceiling breaks it, listed or not. Of a site at or below `LinearWithSorts` nothing is asked. A site above it must be listed:
/// a module's statement once for the pack, a statement of a rule's own query for that rule alone, since two rules' own
/// documents may share a name. A listed site breaks the list when a rule reaches it above the class it is listed at, and when
/// no rule reaches it above `LinearWithSorts` any more: so an entry listed at or below it is stale or rose, never held.
///
/// # Examples
///
/// ```
/// use ryo_query_language::domain::costing::service::held;
/// use ryo_query_language::domain::costing::types::{PackCeiling, PlanCostClass};
///
/// let pack = PackCeiling { ceiling: PlanCostClass::LinearWithSorts, listed: Vec::new() };
/// assert!(held(&[], &pack).is_empty());
/// ```
#[must_use]
pub fn held(costs: &[RuleCost], pack: &PackCeiling) -> Vec<CostBreach> {
    let mut listed: BTreeMap<&SiteOwner, BTreeMap<&SiteOrigin, &ListedSite>> = BTreeMap::new();
    for entry in &pack.listed {
        let _first = listed
            .entry(&entry.owner)
            .or_default()
            .entry(&entry.site)
            .or_insert(entry);
    }
    let mut breaches = Vec::new();
    let mut reached: BTreeMap<SiteOwner, BTreeMap<&SiteOrigin, PlanCostClass>> = BTreeMap::new();
    for RuleCost { rule, cost } in costs {
        for finding in &cost.findings {
            if finding.class > pack.ceiling {
                breaches.push(CostBreach::AboveCeiling(rule.clone(), finding.site.clone()));
            }
            if finding.class <= PlanCostClass::LinearWithSorts {
                continue;
            }
            let owner = match finding.home {
                SiteHome::Module => SiteOwner::Pack,
                SiteHome::Query => SiteOwner::Rule(rule.clone()),
            };
            if listed
                .get(&owner)
                .and_then(|sites| sites.get(&finding.site))
                .is_none()
            {
                breaches.push(CostBreach::Unlisted(rule.clone(), finding.site.clone()));
            }
            let greatest = reached
                .entry(owner)
                .or_default()
                .entry(&finding.site)
                .or_insert(finding.class);
            *greatest = (*greatest).max(finding.class);
        }
    }
    for entry in &pack.listed {
        let greatest = reached
            .get(&entry.owner)
            .and_then(|sites| sites.get(&entry.site))
            .copied();
        if greatest.is_some_and(|class| class > entry.class) {
            breaches.push(CostBreach::Rose(entry.clone()));
        }
        let first = listed
            .get(&entry.owner)
            .and_then(|sites| sites.get(&entry.site))
            .is_some_and(|first| std::ptr::eq(*first, entry));
        if greatest.is_none() || !first {
            breaches.push(CostBreach::Stale(entry.clone()));
        }
    }
    breaches
}
