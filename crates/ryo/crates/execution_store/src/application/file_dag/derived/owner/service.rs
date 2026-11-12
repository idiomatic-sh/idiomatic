//! The owner-issued capability over the derived port.
use crate::outbound::derived::interface::DerivedPort;
use crate::outbound::route_counts::interface::RouteCountsPort;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_derived::domain::request::types::DerivedRequest;
use std::sync::Arc;

/// The owner-issued capability over the derived port. Holders outside the owner can ask the
/// semantics to prepare a contract; only the owner can reach the port, so no holder can build,
/// and every view is built through admission, the ledger and a lease.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
/// use ryo_derived::domain::request::types::DerivedRequest;
/// fn prepared(access: &DerivedAccess, request: DerivedRequest) {
///     let _contract = access.prepare(request, Vec::new());
/// }
/// ```
#[derive(Clone, Debug)]
pub struct DerivedAccess {
    port: Arc<dyn DerivedPort>,
    /// The format route accounting writes its documents in, when the edge composed one.
    routes: Option<Arc<dyn RouteCountsPort>>,
}

impl DerivedAccess {
    /// Request-time metadata only: preparing never builds or retains a view.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
    /// use ryo_derived::domain::request::types::DerivedRequest;
    /// fn prepared(access: &DerivedAccess, request: DerivedRequest) {
    ///     let _contract = access.prepare(request, Vec::new());
    /// }
    /// ```
    pub fn prepare(
        &self,
        request: DerivedRequest,
        schemas: Vec<Vec<ryo_relations::domain::relations::types::CellKind>>,
    ) -> Arc<PreparedDerived> {
        Arc::new(self.port.prepare(request, schemas))
    }
    /// The same capability, whose route accounting writes its documents through `routes`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
    /// use ryo_execution_store::outbound::route_counts::interface::RouteCountsPort;
    /// use std::sync::Arc;
    /// fn routed(access: DerivedAccess, routes: Arc<dyn RouteCountsPort>) -> DerivedAccess {
    ///     access.routed(routes)
    /// }
    /// ```
    #[must_use]
    pub fn routed(self, routes: Arc<dyn RouteCountsPort>) -> Self {
        Self {
            routes: Some(routes),
            ..self
        }
    }
    /// The format route accounting writes in, when the edge composed one.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedAccess;
    /// fn accounted(access: &DerivedAccess) -> bool {
    ///     access.routes().is_some()
    /// }
    /// ```
    #[must_use]
    pub fn routes(&self) -> Option<Arc<dyn RouteCountsPort>> {
        self.routes.clone()
    }
    /// The builder is reachable only inside the owner tree.
    pub(crate) fn port(&self) -> &Arc<dyn DerivedPort> {
        &self.port
    }
}

impl From<Arc<dyn DerivedPort>> for DerivedAccess {
    fn from(port: Arc<dyn DerivedPort>) -> Self {
        Self { port, routes: None }
    }
}

impl<T: DerivedPort + 'static> From<Arc<T>> for DerivedAccess {
    fn from(port: Arc<T>) -> Self {
        Self { port, routes: None }
    }
}

use super::acquisition::semantics::storage::service::DerivedStorage;
use super::graph_key::service::RequirementIdentity;
use super::types::{
    AdmissionOrder, BuildCount, BuildCounts, EntryLimit, FilePreparation, HitCount,
    MembershipOrder, PinCount, PopulationCount, RefusalCount, ReleasedEntries, ScopeId, ViewCount,
};
use crate::application::file_dag::accounting::ledger::types::ByteCount;
use crate::application::file_dag::derived::owner::cache::types::CacheBackend;
use crate::application::file_dag::derived::population::service::PopulationId;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::derived_views::types::contract::value::DerivedContract;
use std::{cell::RefCell, sync::Weak};

/// Runtime semantic dependencies that are not columns: candidate filter and exact dense ordering.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::owner::service::ViewContext;
/// fn named(_: Option<&ViewContext>) {}
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ViewContext {
    pub(crate) candidate: Option<super::graph_key::service::PreparedIdentity>,
    pub(crate) literals: Option<Arc<[ryo_values::domain::plan_parts::types::LiteralText]>>,
    pub(crate) membership_order: Option<MembershipOrder>,
}

impl ViewContext {
    /// The context of a membership view whose members keep the dense order `order`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::ViewContext;
    /// use ryo_execution_store::application::file_dag::derived::owner::types::MembershipOrder;
    ///
    /// let ordered = ViewContext::ordered(MembershipOrder(2));
    /// assert_eq!(ordered, ViewContext::ordered(MembershipOrder(2)));
    /// assert_ne!(ordered, ViewContext::ordered(MembershipOrder(3)));
    /// assert_ne!(ordered, ViewContext::default());
    /// ```
    pub fn ordered(order: MembershipOrder) -> Self {
        Self {
            membership_order: Some(order),
            ..Self::default()
        }
    }
    #[cfg(test)]
    pub(crate) fn for_predicate(plan: &Arc<DerivedContract>) -> Self {
        Self {
            candidate: Some(super::graph_key::service::PreparedIdentity::new(plan)),
            ..Self::default()
        }
    }
}

/// The file-local owner of derived views: it acquires, retains, accounts for and releases them.
///
/// # Examples
///
/// ```
/// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedOwner;
/// fn named(_: Option<&DerivedOwner>) {}
/// ```
pub struct DerivedOwner {
    pub(super) identity: Arc<()>,
    pub(super) semantic: DerivedAccess,
    pub(super) next_scope: std::cell::Cell<ScopeId>,
    #[cfg(any(test, feature = "fixtures"))]
    pub(super) last_scoped_storage: RefCell<Weak<DerivedStorage>>,
    pub(super) ledger:
        std::rc::Rc<RefCell<crate::application::file_dag::accounting::ledger::service::Ledger>>,
    pub(super) backend: RefCell<Option<CacheBackend>>,
    pub(super) file_prepared: std::cell::Cell<FilePreparation>,
    pub(super) state: RefCell<State>,
    pub(super) max_entries: EntryLimit,
    /// The route profile its relation owner hands down.
    pub(super) routes: crate::application::file_dag::route_counts::profile::service::RouteProfile,
}

impl DerivedOwner {
    pub(crate) fn new(
        max_entries: EntryLimit,
        semantic: impl Into<DerivedAccess>,
        routes: crate::application::file_dag::route_counts::profile::service::RouteProfile,
    ) -> Self {
        Self {
            identity: Arc::new(()),
            semantic: semantic.into(),
            next_scope: std::cell::Cell::new(ScopeId(0)),
            #[cfg(any(test, feature = "fixtures"))]
            last_scoped_storage: RefCell::new(Weak::new()),
            ledger: std::rc::Rc::new(RefCell::new(
                crate::application::file_dag::accounting::ledger::service::Ledger::default(),
            )),
            state: RefCell::new(State::default()),
            backend: RefCell::new(None),
            file_prepared: std::cell::Cell::new(FilePreparation::Unprepared),
            max_entries,
            routes,
        }
    }
    /// How many views were built, reused and refused.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedOwner;
    /// use ryo_execution_store::application::file_dag::derived::owner::types::BuildCounts;
    /// fn builds(owner: &DerivedOwner) -> u64 {
    ///     let BuildCounts(built, _hits, _refused) = owner.counts();
    ///     built.0
    /// }
    /// ```
    pub fn counts(&self) -> BuildCounts {
        let state = self.state.borrow();
        BuildCounts(state.builds, state.hits, state.refusals)
    }
    /// The working bytes the owner's views hold now.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedOwner;
    /// fn working(owner: &DerivedOwner) -> usize {
    ///     owner.working_bytes().0
    /// }
    /// ```
    pub fn working_bytes(&self) -> ByteCount {
        self.state.borrow().required_bytes
    }
    pub(crate) fn peak_working_bytes(&self) -> ByteCount {
        self.state.borrow().peak_required_bytes
    }
    /// The bytes the owner retains across invocations.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedOwner;
    /// fn retained(owner: &DerivedOwner) -> usize {
    ///     owner.retained_bytes().0
    /// }
    /// ```
    pub fn retained_bytes(&self) -> ByteCount {
        self.state.borrow().retained_bytes
    }
    pub(crate) fn required_backend_overlap(&self) -> ByteCount {
        self.state.borrow().required_backend_bytes
    }
    pub(crate) fn peak_required_views(&self) -> ViewCount {
        self.state.borrow().peak_required_views
    }
    /// The bytes the owner holds in all.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_store::application::file_dag::derived::owner::service::DerivedOwner;
    /// fn owned(owner: &DerivedOwner) -> usize {
    ///     owner.owned_bytes().0
    /// }
    /// ```
    pub fn owned_bytes(&self) -> ByteCount {
        let state = self.state.borrow();
        ByteCount(
            state
                .required_bytes
                .saturating_add(state.retained_bytes)
                .0
                .saturating_sub(state.overlap_bytes.0),
        )
    }
    pub(crate) fn metadata_bytes(&self) -> ByteCount {
        use crate::application::file_dag::accounting::ledger::{
            capacity::{service::saturating, service::table},
            types::SlotCount,
        };
        let state = self.state.borrow();
        let plans = state
            .entries
            .keys()
            .map(|key| key.plan.metadata_bytes().0)
            .sum::<usize>();
        let spare = state
            .spare_dependencies
            .iter()
            .map(|keys| saturating::<Key>(SlotCount(keys.capacity())).0)
            .sum::<usize>();
        let dependent_keys = state
            .dependents
            .values()
            .map(|keys| saturating::<Key>(SlotCount(keys.capacity())).0)
            .sum::<usize>();
        ByteCount(
            plans
                + table::<(Key, OwnerEntry)>(SlotCount(state.entries.capacity()))
                    .0
                    .saturating_mul(2)
                + table::<(PopulationId, PinCount)>(SlotCount(state.pins.capacity()))
                    .0
                    .saturating_mul(2)
                + table::<(PopulationId, Vec<Key>)>(SlotCount(state.dependents.capacity()))
                    .0
                    .saturating_mul(2)
                + dependent_keys
                + saturating::<Vec<Key>>(SlotCount(state.spare_dependencies.capacity())).0
                + spare,
        )
    }
}

#[cfg(test)]
impl DerivedOwner {
    pub(crate) fn released_entries(&self) -> ReleasedEntries {
        self.state.borrow().released_entries
    }
}

pub(super) fn link(state: &mut State, key: Key, populations: &[PopulationId]) {
    for &population in populations {
        state
            .dependents
            .entry(population)
            .or_insert_with(|| state.spare_dependencies.pop().unwrap_or_default())
            .push(key.clone());
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Key {
    pub(crate) plan: RequirementIdentity,
    pub(crate) populations: [PopulationId; 2],
    pub(crate) len: PopulationCount,
    pub(crate) context: ViewContext,
}

pub(super) struct OwnedView {
    pub(crate) storage: Arc<DerivedStorage>,
    pub(crate) _lease:
        Arc<crate::application::file_dag::accounting::ledger::service::ResourceLease>,
}

pub(super) struct OwnerEntry {
    pub(crate) admission: AdmissionOrder,
    pub(crate) _plan: Arc<DerivedContract>,
    pub(crate) weak: Weak<DerivedStorage>,
    pub(crate) required: Option<OwnedView>,
    pub(crate) retained: Option<OwnedView>,
    pub(crate) backings: Arc<[crate::application::file_dag::accounting::ledger::types::Backing]>,
    pub(crate) lease:
        Weak<crate::application::file_dag::accounting::ledger::service::ResourceLease>,
    pub(crate) bytes: ByteCount,
    pub(crate) working: ByteCount,
    pub(crate) backend: ByteCount,
}

#[derive(Default)]
pub(super) struct State {
    pub(crate) next_admission: AdmissionOrder,
    pub(crate) entries: hashbrown::HashMap<Key, OwnerEntry>,
    pub(crate) pins: FxHashMap<PopulationId, PinCount>,
    pub(crate) dependents: FxHashMap<PopulationId, Vec<Key>>,
    pub(crate) spare_dependencies: Vec<Vec<Key>>,
    pub(crate) released_entries: ReleasedEntries,
    pub(crate) builds: BuildCount,
    pub(crate) hits: HitCount,
    pub(crate) refusals: RefusalCount,
    pub(crate) required_bytes: ByteCount,
    pub(crate) peak_required_bytes: ByteCount,
    pub(crate) retained_bytes: ByteCount,
    pub(crate) overlap_bytes: ByteCount,
    pub(crate) required_views: ViewCount,
    pub(crate) peak_required_views: ViewCount,
    pub(crate) required_backend_bytes: ByteCount,
}
