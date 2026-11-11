//! The rule, cache population and eviction helpers the rule compilation cases share.
use super::super::constants::COMPILED_PLANS;
use super::super::errors::RuleCompilationError;
use super::super::service::CompiledPlans;
use super::super::service::RuleCompilation;
use crate::outbound::query_language::errors::QueryLanguageError;
use crate::outbound::query_language::fixtures::fake::{QueryLanguageFake, REFUSED};
use crate::outbound::query_language::interface::QueryLanguagePort;
use ryo_values::domain::descriptions::types::ValidateReport;
use ryo_values::domain::hosted::types::HostedRelation;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::borrow::Cow;
use std::sync::PoisonError;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, atomic::Ordering};

/// A legitimate parameter-free rule with exact query identity.
pub(super) fn rule(id: &str, text: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryBundle::from(QueryText(text.into())),
        applicability: RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

/// The shared cache's actual population after an operation.
pub(super) fn population(plans: &CompiledPlans) -> usize {
    plans
        .0
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .plans
        .len()
}

/// What a compilation scenario observed: the shared cache's population, the compiler's calls, and each further
/// observation by name (a relation as 1 when it held, 0 when not).
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Compiled {
    pub(super) kept: usize,
    pub(super) calls: usize,
    pub(super) observed: Cow<'static, [(&'static str, usize)]>,
}

/// Hold the returned invocation plans while FIFO eviction drops only the oldest shared entry.
pub(super) fn bounded() -> Result<Compiled, RuleCompilationError> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let rules: Vec<_> = (0..=COMPILED_PLANS)
        .map(|index| rule(&format!("rule-{index}"), "select"))
        .collect();
    let answer = service.compile(&rules, &[])?;
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let held = |id: &str| usize::from(kept.plans.contains_key(&RuleId(id.into())));
    Ok(Compiled {
        kept: kept.plans.len(),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: vec![
            ("answered", answer.plans.len()),
            ("rule-0 kept", held("rule-0")),
            ("rule-1 kept", held("rule-1")),
        ]
        .into(),
    })
}

/// Distinct ids exercise the shared cache across several complete requests.
pub(super) fn changed_ids() -> Result<Compiled, RuleCompilationError> {
    const PER_RUN: usize = 1024;
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let (mut most, mut short, mut over) = (0, 0, 0);
    for run in 0..COMPILED_PLANS / PER_RUN + 2 {
        let rules: Vec<_> = (0..PER_RUN)
            .map(|place| rule(&format!("{run}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        most = most.max(population(&plans));
        short += usize::from(answer.plans.len() != PER_RUN);
        over += usize::from(population(&plans) > COMPILED_PLANS);
    }
    Ok(Compiled {
        kept: population(&plans),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: vec![
            ("most kept", most),
            ("requests answered short", short),
            ("requests leaving more than the cap", over),
        ]
        .into(),
    })
}

/// An invocation retains every answer even when its own ids overflow the shared cache.
pub(super) fn oversized_requests() -> Result<Compiled, RuleCompilationError> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut observed = vec![];
    for (tag, count) in [("big", COMPILED_PLANS + 1), ("bigger", COMPILED_PLANS * 2)] {
        let rules: Vec<_> = (0..count)
            .map(|place| rule(&format!("{tag}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        observed.push((tag, answer.plans.len()));
        observed.push(("kept after it", population(&plans)));
    }
    Ok(Compiled {
        kept: population(&plans),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: observed.into(),
    })
}

/// FIFO eviction after later requests preserves every entry except the oldest.
pub(super) fn separate_request_eviction() -> Result<Compiled, RuleCompilationError> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut observed = vec![];
    for (tag, count) in [
        ("old", COMPILED_PLANS / 2),
        ("newer", COMPILED_PLANS - COMPILED_PLANS / 2),
        ("latest", 1),
    ] {
        let rules: Vec<_> = (0..count)
            .map(|place| rule(&format!("{tag}-{place}"), "one"))
            .collect();
        let answer = service.compile(&rules, &[])?;
        observed.push((tag, answer.plans.len()));
    }
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let named = |tag: &str, place| {
        usize::from(
            kept.plans
                .contains_key(&RuleId(format!("{tag}-{place}").into())),
        )
    };
    observed.extend([
        ("old-0 kept", named("old", 0)),
        ("old-1 kept", named("old", 1)),
        ("newer-0 kept", named("newer", 0)),
        (
            "last newer kept",
            named("newer", COMPILED_PLANS - COMPILED_PLANS / 2 - 1),
        ),
        ("latest-0 kept", named("latest", 0)),
    ]);
    Ok(Compiled {
        kept: kept.plans.len(),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: observed.into(),
    })
}

/// The real bounded cache compares exact catalog contents, never allocation identity.
pub(super) fn module_catalog_identity() -> Result<Compiled, RuleCompilationError> {
    use ryo_values::domain::program::service::{document_source, query_bundle, source_id};
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let module = source_id(ryo_values::domain::query_tokens::types::QuerySpelling(
        "shared/module",
    ))
    .expect("nonblank module id");
    let mut asked = rule("catalog", "select");
    asked.query = query_bundle(
        asked.query.root,
        [document_source(module.clone(), "original bytes\nλ")],
    )
    .expect("unique module identity");
    let snapshot = asked.query.clone();
    let mut observed = vec![
        (
            "the snapshot shares the modules",
            usize::from(Arc::ptr_eq(&snapshot.modules, &asked.query.modules)),
        ),
        (
            "the snapshot shares the root text",
            usize::from(Arc::ptr_eq(&snapshot.root.text.0, &asked.query.root.text.0)),
        ),
    ];
    let _first = service.compile(&[asked.clone()], &[])?;
    let mut equal_contents = asked.clone();
    equal_contents.query.modules = Arc::new((*asked.query.modules).clone());
    observed.push((
        "equal contents share the modules",
        usize::from(Arc::ptr_eq(
            &equal_contents.query.modules,
            &asked.query.modules,
        )),
    ));
    let _equal_snapshot_hit = service.compile(&[equal_contents], &[])?;
    observed.push((
        "calls after equal contents",
        compiler.calls.load(Ordering::SeqCst),
    ));
    Arc::make_mut(&mut asked.query.modules)
        .get_mut(&module)
        .unwrap()
        .text = "changed bytes".into();
    observed.push((
        "the snapshot keeps the original bytes",
        usize::from(&*snapshot.modules[&module].text == "original bytes\nλ"),
    ));
    let _changed_bytes_miss = service.compile(&[asked.clone()], &[])?;
    observed.push((
        "calls after changed bytes",
        compiler.calls.load(Ordering::SeqCst),
    ));
    let renamed_id = source_id(ryo_values::domain::query_tokens::types::QuerySpelling(
        "shared/renamed",
    ))
    .expect("nonblank renamed id");
    Arc::make_mut(&mut asked.query.modules)
        .get_mut(&module)
        .unwrap()
        .id = renamed_id.clone();
    let _changed_document_id_miss = service.compile(&[asked.clone()], &[])?;
    observed.push((
        "calls after a changed document id",
        compiler.calls.load(Ordering::SeqCst),
    ));
    let renamed = Arc::make_mut(&mut asked.query.modules)
        .remove(&module)
        .unwrap();
    let _absent = Arc::make_mut(&mut asked.query.modules).insert(renamed_id, renamed);
    let _changed_id_miss = service.compile(&[asked.clone()], &[])?;
    let _stable_snapshot_hit = service.compile(&[asked.clone()], &[])?;
    let kept = plans.0.lock().unwrap_or_else(PoisonError::into_inner);
    let cached = &kept.plans[&asked.id].query;
    observed.extend([
        (
            "the cache shares the modules",
            usize::from(Arc::ptr_eq(&cached.modules, &asked.query.modules)),
        ),
        (
            "the cache shares the root text",
            usize::from(Arc::ptr_eq(&cached.root.text.0, &asked.query.root.text.0)),
        ),
    ]);
    Ok(Compiled {
        kept: kept.plans.len(),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: observed.into(),
    })
}

/// Repeated refused compiles never become a cache hit, and a later valid query still succeeds.
pub(super) fn refused_program() -> Result<Compiled, RuleCompilationError> {
    let compiler = Arc::new(QueryLanguageFake::default());
    let plans = CompiledPlans::default();
    let service = RuleCompilation::new(Arc::<QueryLanguageFake>::clone(&compiler))
        .compiling_into(plans.clone());
    let mut observed = vec![];
    for _ in 0_u8..2_u8 {
        let error = service.compile(&[rule("owner", REFUSED)], &[]).err();
        observed.push((
            "refused for its owner",
            usize::from(
                matches!(error, Some(RuleCompilationError::Query { id, .. }) if id == "owner"),
            ),
        ));
        observed.push(("kept after the refusal", population(&plans)));
    }
    let _answer = service.compile(&[rule("owner", "select")], &[])?;
    Ok(Compiled {
        kept: population(&plans),
        calls: compiler.calls.load(Ordering::SeqCst),
        observed: observed.into(),
    })
}

/// A substituted compiler that proves no service cache lock crosses the port call.
struct LockCheckingCompiler {
    /// The same session state the service owns.
    plans: CompiledPlans,
    /// Existing compiler fake for typed answers and observations.
    compiler: QueryLanguageFake,
    /// The compiler calls made while a cache lock was held.
    crossed: AtomicUsize,
}
impl QueryLanguagePort for LockCheckingCompiler {
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError> {
        self.compiler.validate(bundle, hosted, observations)
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        // A cache lock held across the call is counted, not waited for.
        let held = self.plans.0.try_lock().err().map_or(0, |_| 1);
        let _counted = self.crossed.fetch_add(held, Ordering::SeqCst);
        self.compiler.compile(bundle, hosted, observations)
    }
}

/// Execute through the lock-checking port rather than inspecting implementation syntax.
pub(super) fn unlocked_port() -> Result<Compiled, RuleCompilationError> {
    let plans = CompiledPlans::default();
    let compiler = Arc::new(LockCheckingCompiler {
        plans: plans.clone(),
        compiler: QueryLanguageFake::default(),
        crossed: AtomicUsize::new(0),
    });
    let service = RuleCompilation::new(Arc::<LockCheckingCompiler>::clone(&compiler))
        .compiling_into(plans.clone());
    let _answer = service.compile(&[rule("lock", "select")], &[])?;
    Ok(Compiled {
        kept: population(&plans),
        calls: compiler.compiler.calls.load(Ordering::SeqCst),
        observed: vec![(
            "calls under a held cache lock",
            compiler.crossed.load(Ordering::SeqCst),
        )]
        .into(),
    })
}
